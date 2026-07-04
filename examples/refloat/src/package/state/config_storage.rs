use super::RefloatPackageState;
use crate::config::RefloatConfigImage;
use crate::domain::{RefloatMode, RefloatRunState};

impl RefloatPackageState {
    pub(in crate::package) fn serialized_config(&self) -> &[u8; 276] {
        // C map: `get_cfg(..., is_default=false)` serializes the current
        // `d->float_conf` image at `third_party/refloat/src/main.c:2335-2356`.
        self.serialized_config.as_bytes()
    }

    #[cfg(test)]
    pub(in crate::package) fn replace_serialized_config_for_test(
        &mut self,
        config: crate::config::RefloatConfigImage,
    ) {
        self.serialized_config = config;
    }

    #[cfg(test)]
    pub(in crate::package) fn balance_config_for_test(
        &self,
    ) -> crate::config::RefloatBalanceConfig<'_> {
        self.serialized_config.balance()
    }

    pub(in crate::package) fn store_serialized_config(&mut self, config: &[u8]) -> bool {
        let Some(mut config) = RefloatConfigImage::from_serialized(config) else {
            return false;
        };

        let ride_state = self.all_data_payloads.base().status().ride_state();
        // Upstream refuses VESC Tool writes outside `MODE_NORMAL` before
        // deserializing/storing at `third_party/refloat/src/main.c:2362-2368`.
        if !matches!(ride_state.mode(), RefloatMode::Normal) {
            return false;
        }

        // Upstream clears `d->float_conf.disabled` while running at
        // `third_party/refloat/src/main.c:2369-2372`; `disabled` is
        // serialized from `third_party/refloat/src/conf/settings.xml:3890-3902`
        // at byte 243.
        if matches!(ride_state.run_state(), RefloatRunState::Running) {
            config.editor().keep_enabled_while_running();
        }
        // Upstream clears `d->float_conf.meta.is_default` for every write at
        // `third_party/refloat/src/main.c:2375-2377`; `meta.is_default`
        // is serialized from `third_party/refloat/src/conf/settings.xml:3903-3914`
        // at byte 275.
        config.editor().clear_meta_is_default();
        self.serialized_config = config;
        // After a successful write, C calls `configure(d)` at
        // `third_party/refloat/src/main.c:2380-2382`, which refreshes the balance filter KP at
        // `third_party/refloat/src/main.c:158-160`.
        self.refresh_balance_filter_config();
        true
    }

    fn refresh_balance_filter_config(&mut self) {
        // C map: `reconfigure(d)` refreshes Mahony filter gains through
        // `balance_filter_configure` at `third_party/refloat/src/main.c:154-160`.
        self.balance_filter
            .configure_from(self.serialized_config.filter());
    }

    #[cfg(any(test, target_arch = "arm"))]
    pub(crate) fn configured_loop_time_us(&self) -> u32 {
        // Upstream `configure(d)` stores `1e6 / d->float_conf.hertz` at
        // `third_party/refloat/src/main.c:190-191`, then `refloat_thd`
        // sleeps that value at `third_party/refloat/src/main.c:1080`.
        // Target Rust must not panic if config bytes are corrupt, so keep the
        // startup default instead of dividing by zero.
        self.serialized_config.startup().loop_time_us()
    }
}
