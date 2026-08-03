use super::FloatOutBoyAppDataCommand;

#[test]
fn every_refloat_command_id_round_trips_through_the_typed_model() {
    for (command, id) in [
        (FloatOutBoyAppDataCommand::Info, 0),
        (FloatOutBoyAppDataCommand::GetRealtimeData, 1),
        (FloatOutBoyAppDataCommand::RuntimeTune, 2),
        (FloatOutBoyAppDataCommand::TuneDefaults, 3),
        (FloatOutBoyAppDataCommand::ConfigSave, 4),
        (FloatOutBoyAppDataCommand::ConfigRestore, 5),
        (FloatOutBoyAppDataCommand::TuneOther, 6),
        (FloatOutBoyAppDataCommand::RcMove, 7),
        (FloatOutBoyAppDataCommand::Booster, 8),
        (FloatOutBoyAppDataCommand::PrintInfo, 9),
        (FloatOutBoyAppDataCommand::GetAllData, 10),
        (FloatOutBoyAppDataCommand::Experiment, 11),
        (FloatOutBoyAppDataCommand::Lock, 12),
        (FloatOutBoyAppDataCommand::HandTest, 13),
        (FloatOutBoyAppDataCommand::TuneTilt, 14),
        (FloatOutBoyAppDataCommand::LightsControl, 20),
        (FloatOutBoyAppDataCommand::Flywheel, 22),
        (FloatOutBoyAppDataCommand::LcmPoll, 24),
        (FloatOutBoyAppDataCommand::LcmLightInfo, 25),
        (FloatOutBoyAppDataCommand::LcmLightControl, 26),
        (FloatOutBoyAppDataCommand::LcmDeviceInfo, 27),
        (FloatOutBoyAppDataCommand::ChargingState, 28),
        (FloatOutBoyAppDataCommand::LcmGetBattery, 29),
        (FloatOutBoyAppDataCommand::RealtimeData, 31),
        (FloatOutBoyAppDataCommand::RealtimeDataIds, 32),
        (FloatOutBoyAppDataCommand::AlertsList, 35),
        (FloatOutBoyAppDataCommand::AlertsControl, 36),
        (FloatOutBoyAppDataCommand::DataRecordRequest, 41),
        (FloatOutBoyAppDataCommand::DataRecordHeader, 42),
        (FloatOutBoyAppDataCommand::DataRecordData, 43),
        (FloatOutBoyAppDataCommand::LcmDebug, 99),
    ] {
        assert_eq!(command.id(), id);
        assert_eq!(FloatOutBoyAppDataCommand::try_from_id(id), Ok(command));
    }
}
