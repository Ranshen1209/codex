use codex_features::FEATURES;
use codex_protocol::account::PlanType;
use lazy_static::lazy_static;
use rand::Rng;

const FAST_TOOLTIP: &str =
    "*New* Use **/fast** to enable our fastest inference with increased plan usage.";
const OTHER_TOOLTIP: &str = "*New* Build faster with Sakrylle.";
const FREE_GO_TOOLTIP: &str =
    "*New* For a limited time, Sakrylle is included in your plan for free – let’s build together.";

const RAW_TOOLTIPS: &str = include_str!("../tooltips.txt");

lazy_static! {
    static ref TOOLTIPS: Vec<&'static str> = RAW_TOOLTIPS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    static ref ALL_TOOLTIPS: Vec<&'static str> = {
        let mut tips = Vec::new();
        tips.extend(TOOLTIPS.iter().copied());
        tips.extend(experimental_tooltips());
        tips
    };
}

fn experimental_tooltips() -> Vec<&'static str> {
    FEATURES
        .iter()
        .filter_map(|spec| spec.stage.experimental_announcement())
        .collect()
}

/// Pick a random tooltip to show to the user when starting Sakrylle.
pub(crate) fn get_tooltip(plan: Option<PlanType>, fast_mode_enabled: bool) -> Option<String> {
    let mut rng = rand::rng();

    // Leave small chance for a random tooltip to be shown.
    if rng.random_ratio(8, 10) {
        match plan {
            Some(plan_type)
                if matches!(
                    plan_type,
                    PlanType::Plus | PlanType::Enterprise | PlanType::Pro | PlanType::ProLite
                ) || plan_type.is_team_like()
                    || plan_type.is_business_like() =>
            {
                if let Some(tooltip) = pick_paid_tooltip(fast_mode_enabled) {
                    return Some(tooltip.to_string());
                }
            }
            Some(PlanType::Go) | Some(PlanType::Free) => {
                return Some(FREE_GO_TOOLTIP.to_string());
            }
            _ => {
                return Some(OTHER_TOOLTIP.to_string());
            }
        }
    }

    pick_tooltip(&mut rng).map(str::to_string)
}

/// Paid users get the Fast-mode promo unless they already have Fast mode enabled,
/// in which case they fall through to the generic tooltip pool.
fn pick_paid_tooltip(fast_mode_enabled: bool) -> Option<&'static str> {
    if fast_mode_enabled {
        None
    } else {
        Some(FAST_TOOLTIP)
    }
}

fn pick_tooltip<R: Rng + ?Sized>(rng: &mut R) -> Option<&'static str> {
    if ALL_TOOLTIPS.is_empty() {
        None
    } else {
        ALL_TOOLTIPS
            .get(rng.random_range(0..ALL_TOOLTIPS.len()))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn random_tooltip_returns_some_tip_when_available() {
        let mut rng = StdRng::seed_from_u64(42);
        assert!(pick_tooltip(&mut rng).is_some());
    }

    #[test]
    fn random_tooltip_is_reproducible_with_seed() {
        let expected = {
            let mut rng = StdRng::seed_from_u64(7);
            pick_tooltip(&mut rng)
        };

        let mut rng = StdRng::seed_from_u64(7);
        assert_eq!(expected, pick_tooltip(&mut rng));
    }

    #[test]
    fn paid_tooltip_promotes_fast_mode_unless_enabled() {
        assert_eq!(
            Some(FAST_TOOLTIP),
            pick_paid_tooltip(/*fast_mode_enabled*/ false)
        );
        assert_eq!(None, pick_paid_tooltip(/*fast_mode_enabled*/ true));
    }
}
