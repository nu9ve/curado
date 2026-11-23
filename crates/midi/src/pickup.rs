/// Takes ownership of an absolute parameter only after the control reaches its current value.
/// The caller resets this value when the range or parameter owner changes.
#[derive(Default)]
pub struct Pickup {
    previous: Option<f64>,
    captured: bool,
}

impl Pickup {
    pub fn apply(&mut self, position: f64, target: f64, tolerance: f64) -> Option<f64> {
        let previous = self.previous.replace(position);
        if self.captured {
            return Some(position);
        }
        if !(0. ..=1.).contains(&target) {
            return None;
        }
        let crossed = previous.is_some_and(|prev| {
            (prev <= target && position >= target) || (prev >= target && position <= target)
        });
        if crossed || (position - target).abs() <= tolerance {
            self.captured = true;
            return Some(target);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Pickup;

    #[test]
    fn crossing_captures_at_the_target_before_following() {
        let mut pickup = Pickup::default();
        assert_eq!(pickup.apply(0.8, 0.5, 0.004), None);
        assert_eq!(pickup.apply(0.65, 0.5, 0.004), None);
        assert_eq!(pickup.apply(0.48, 0.5, 0.004), Some(0.5));
        assert_eq!(pickup.apply(0.44, 0.5, 0.004), Some(0.44));
    }

    #[test]
    fn outside_the_range_stays_uncaptured() {
        let mut pickup = Pickup::default();
        assert_eq!(pickup.apply(0.6, 1.1, 0.004), None);
        assert_eq!(pickup.apply(1., 1.1, 0.004), None);
        assert_eq!(pickup.apply(0.5, 0.5, 0.004), Some(0.5));
    }
}
