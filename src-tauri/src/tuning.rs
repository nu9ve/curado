//! One bounded worker, at most two raw source clips and two prepared results.
use crate::AppState;
use curado_audio::{
    decode::Clip,
    tuning::{self, TunedClip},
};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

pub fn run(app: Arc<AppState>) {
    let mut sources: [Option<(u64, Arc<Clip>)>; 2] = [None, None];
    let mut failed = [None; 2];
    let mut last_generation = u64::MAX;
    while app.running.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(30));
        let generation = app.generation.load(Ordering::Acquire);
        if generation != last_generation {
            sources = [None, None];
            failed = [None; 2];
            last_generation = generation;
        }
        let handle = app
            .output
            .lock()
            .unwrap()
            .as_ref()
            .map(|o| o.handle.clone());
        let Some(handle) = handle else {
            continue;
        };
        let snap = handle.snapshot();
        for (i, source) in sources.iter_mut().enumerate() {
            if source
                .as_ref()
                .is_some_and(|(t, _)| *t != snap.slots[i].load_ticket)
            {
                *source = None;
            }
        }
        let jobs: [bool; 2] = std::array::from_fn(|i| {
            let s = &snap.slots[i];
            let tag = Some((s.load_ticket, s.tune_revision));
            s.id != 0 && s.tune_revision != s.tuned_revision && failed[i] != tag
        });
        if !jobs.iter().any(|v| *v) {
            continue;
        }
        let current = || {
            app.running.load(Ordering::Relaxed)
                && app.generation.load(Ordering::Acquire) == generation
                && (0..2).all(|i| {
                    !jobs[i]
                        || handle.tuning_current(
                            i,
                            snap.slots[i].load_ticket,
                            snap.slots[i].tune_revision,
                            snap.slots[i].tuning,
                        )
                })
        };
        // Coalesce fast controls before allocating a full prepared clip.
        for _ in 0..10 {
            if !current() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if !current() {
            continue;
        }
        let mut batch: [Option<TunedClip>; 2] = [None, None];
        let result = (|| -> Result<(), String> {
            for i in 0..2 {
                if !jobs[i] {
                    continue;
                }
                let s = &snap.slots[i];
                if sources[i].as_ref().is_none_or(|(t, _)| *t != s.load_ticket) {
                    let _decode = app.decode_gate.lock().unwrap();
                    if !current() {
                        return Err("Petición sustituida".into());
                    }
                    sources[i] = Some((s.load_ticket, app.decode_track(s.id)?));
                }
                let source = &sources[i].as_ref().unwrap().1;
                let clip = tuning::prepare(source, s.tuning, || !current())?;
                batch[i] = Some(TunedClip {
                    clip,
                    load_ticket: s.load_ticket,
                    revision: s.tune_revision,
                    tuning: s.tuning,
                });
            }
            // Serialize with device replacement; never submit to an obsolete output.
            let waiting = std::time::Instant::now();
            let ticket = loop {
                let output = app.output.lock().unwrap();
                if !current() || output.is_none() {
                    return Err("Salida o petición sustituida".into());
                }
                let result = handle.apply_tuning(batch.clone());
                drop(output);
                match result {
                    Ok(ticket) => break ticket,
                    Err(error) if waiting.elapsed() > Duration::from_secs(2) => return Err(error),
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            };
            let started = std::time::Instant::now();
            while !handle.tuning_acknowledged(ticket) {
                if !app.running.load(Ordering::Relaxed)
                    || app.generation.load(Ordering::Acquire) != generation
                {
                    return Err("Salida sustituida".into());
                }
                if started.elapsed() > Duration::from_secs(2) {
                    return Err("La salida no confirmó el ajuste; recupera el dispositivo".into());
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            if current() {
                let applied = handle.snapshot();
                if (0..2).any(|i| {
                    jobs[i] && applied.slots[i].tuned_revision != snap.slots[i].tune_revision
                }) {
                    return Err(
                        "El audio preparado no coincide con la pista cargada; vuelve a cargarla"
                            .into(),
                    );
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                *app.tuning_error.lock().unwrap() = None;
            }
            Err(error) if current() => {
                for i in 0..2 {
                    if jobs[i] {
                        failed[i] = Some((snap.slots[i].load_ticket, snap.slots[i].tune_revision));
                    }
                }
                *app.tuning_error.lock().unwrap() = Some(error);
            }
            Err(_) => {}
        }
    }
}
