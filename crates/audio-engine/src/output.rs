use crate::{engine, EngineHandle, Renderer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::sync::mpsc;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputInfo {
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: Option<u32>,
    pub backend: String,
}
pub struct Output {
    pub handle: EngineHandle,
    pub info: OutputInfo,
    stop: Option<mpsc::Sender<()>>,
}
impl Drop for Output {
    fn drop(&mut self) {
        self.stop.take();
    }
}
pub fn devices() -> Result<Vec<String>, String> {
    cpal::default_host()
        .output_devices()
        .map_err(|e| e.to_string())?
        .map(|d| d.name().map_err(|e| e.to_string()))
        .collect()
}
pub fn start(device_name: Option<String>, frames: Option<u32>) -> Result<Output, String> {
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (stop_tx, stop_rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("curado-device".into())
        .spawn(move || {
            let create = || -> Result<(_, _, _), String> {
                let host = cpal::default_host();
                let device = if let Some(name) = device_name {
                    host.output_devices()
                        .map_err(|e| e.to_string())?
                        .find(|d| d.name().ok().as_ref() == Some(&name))
                        .ok_or("Dispositivo no disponible")?
                } else {
                    host.default_output_device()
                        .ok_or("No hay dispositivo de salida de audio")?
                };
                let supported = device.default_output_config().map_err(|e| e.to_string())?;
                let format = supported.sample_format();
                let mut config: cpal::StreamConfig = supported.into();
                if let Some(f) = frames {
                    if !(64..=4096).contains(&f) {
                        return Err("Buffer inválido".into());
                    }
                    config.buffer_size = cpal::BufferSize::Fixed(f);
                }
                let (handle, renderer) = engine(config.sample_rate.0);
                let info = OutputInfo {
                    device: device.name().map_err(|e| e.to_string())?,
                    sample_rate: config.sample_rate.0,
                    channels: config.channels,
                    buffer_frames: frames,
                    backend: format!("{:?}", host.id()),
                };
                let stream = match format {
                    cpal::SampleFormat::F32 => {
                        stream::<f32>(&device, &config, renderer, handle.clone())
                    }
                    cpal::SampleFormat::F64 => {
                        stream::<f64>(&device, &config, renderer, handle.clone())
                    }
                    cpal::SampleFormat::I16 => {
                        stream::<i16>(&device, &config, renderer, handle.clone())
                    }
                    cpal::SampleFormat::I32 => {
                        stream::<i32>(&device, &config, renderer, handle.clone())
                    }
                    cpal::SampleFormat::U16 => {
                        stream::<u16>(&device, &config, renderer, handle.clone())
                    }
                    other => return Err(format!("Formato de salida no soportado: {other}")),
                }
                .map_err(|e| e.to_string())?;
                stream.play().map_err(|e| e.to_string())?;
                Ok((handle, info, stream))
            };
            match create() {
                Ok((handle, info, stream)) => {
                    let _ = ready_tx.send(Ok((handle, info)));
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            }
        })
        .map_err(|e| e.to_string())?;
    let (handle, info) = ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(Output {
        handle,
        info,
        stop: Some(stop_tx),
    })
}
fn stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut renderer: Renderer,
    handle: EngineHandle,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| renderer.render(data, channels),
        move |_| handle.fail(),
        None,
    )
}
