//! FOB app-data command wire-ID characterization tests.

use super::FloatOutBoyAppDataCommand;

#[test]
fn protocol_reference_command_table_matches_typed_discriminants() {
    let reference = include_str!("../../../../docs/float-out-boy-protocol.md");
    for (command, id) in [
        ("Info", FloatOutBoyAppDataCommand::Info),
        (
            "GetRealtimeData",
            FloatOutBoyAppDataCommand::GetRealtimeData,
        ),
        ("RuntimeTune", FloatOutBoyAppDataCommand::RuntimeTune),
        ("TuneDefaults", FloatOutBoyAppDataCommand::TuneDefaults),
        ("ConfigSave", FloatOutBoyAppDataCommand::ConfigSave),
        ("ConfigRestore", FloatOutBoyAppDataCommand::ConfigRestore),
        ("TuneOther", FloatOutBoyAppDataCommand::TuneOther),
        ("Booster", FloatOutBoyAppDataCommand::Booster),
        ("PrintInfo", FloatOutBoyAppDataCommand::PrintInfo),
        ("GetAllData", FloatOutBoyAppDataCommand::GetAllData),
        ("Experiment", FloatOutBoyAppDataCommand::Experiment),
        ("Lock", FloatOutBoyAppDataCommand::Lock),
        ("HandTest", FloatOutBoyAppDataCommand::HandTest),
        ("TuneTilt", FloatOutBoyAppDataCommand::TuneTilt),
        ("Remote", FloatOutBoyAppDataCommand::Remote),
        ("LightsControl", FloatOutBoyAppDataCommand::LightsControl),
        ("Flywheel", FloatOutBoyAppDataCommand::Flywheel),
        ("LcmPoll", FloatOutBoyAppDataCommand::LcmPoll),
        ("LcmLightInfo", FloatOutBoyAppDataCommand::LcmLightInfo),
        (
            "LcmLightControl",
            FloatOutBoyAppDataCommand::LcmLightControl,
        ),
        ("LcmDeviceInfo", FloatOutBoyAppDataCommand::LcmDeviceInfo),
        ("ChargingState", FloatOutBoyAppDataCommand::ChargingState),
        ("LcmGetBattery", FloatOutBoyAppDataCommand::LcmGetBattery),
        ("RealtimeData", FloatOutBoyAppDataCommand::RealtimeData),
        (
            "RealtimeDataIds",
            FloatOutBoyAppDataCommand::RealtimeDataIds,
        ),
        (
            "RealtimeDataSelected",
            FloatOutBoyAppDataCommand::RealtimeDataSelected,
        ),
        ("AlertsList", FloatOutBoyAppDataCommand::AlertsList),
        ("AlertsControl", FloatOutBoyAppDataCommand::AlertsControl),
        (
            "DataRecordRequest",
            FloatOutBoyAppDataCommand::DataRecordRequest,
        ),
        (
            "DataRecordHeader",
            FloatOutBoyAppDataCommand::DataRecordHeader,
        ),
        ("DataRecordData", FloatOutBoyAppDataCommand::DataRecordData),
        ("LcmDebug", FloatOutBoyAppDataCommand::LcmDebug),
    ] {
        let row = std::format!("| `{command}` | `{}` |", id.id());
        assert!(
            reference.contains(&row),
            "protocol reference is missing `{row}`"
        );
    }
}

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
        (FloatOutBoyAppDataCommand::Booster, 8),
        (FloatOutBoyAppDataCommand::PrintInfo, 9),
        (FloatOutBoyAppDataCommand::GetAllData, 10),
        (FloatOutBoyAppDataCommand::Experiment, 11),
        (FloatOutBoyAppDataCommand::Lock, 12),
        (FloatOutBoyAppDataCommand::HandTest, 13),
        (FloatOutBoyAppDataCommand::TuneTilt, 14),
        (FloatOutBoyAppDataCommand::Remote, 15),
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
        (FloatOutBoyAppDataCommand::RealtimeDataSelected, 33),
        (FloatOutBoyAppDataCommand::AlertsList, 35),
        (FloatOutBoyAppDataCommand::AlertsControl, 36),
        (FloatOutBoyAppDataCommand::DataRecordRequest, 41),
        (FloatOutBoyAppDataCommand::DataRecordHeader, 42),
        (FloatOutBoyAppDataCommand::DataRecordData, 43),
        (FloatOutBoyAppDataCommand::LcmDebug, 99),
    ] {
        assert_eq!(command.id(), id);
        assert_eq!(FloatOutBoyAppDataCommand::try_from(id), Ok(command));
    }
}
