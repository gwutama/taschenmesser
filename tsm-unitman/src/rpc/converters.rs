use log::{warn};
use tsm_ipc::tsm_unitman_rpc;
use protobuf::EnumOrUnknown;

use crate::unit;


pub fn convert_units_to_proto(units: &Vec<unit::UnitRef>) -> Vec<tsm_unitman_rpc::Unit> {
    let mut proto_units = Vec::new();
    for unit in units {
        match convert_unit_to_proto(unit) {
            Ok(proto_unit) => proto_units.push(proto_unit),
            Err(error) => warn!("{}", error),
        }
    }
    proto_units
}


pub fn convert_unit_to_proto(unit: &unit::UnitRef) -> Result<tsm_unitman_rpc::Unit, String> {
    match unit.try_lock() {
        Ok(unit) => {
            let mut proto_unit = tsm_unitman_rpc::Unit::new();

            proto_unit.name = unit.get_name().clone();
            proto_unit.executable = unit.get_executable().clone();
            proto_unit.arguments = unit.get_arguments().clone();
            proto_unit.restart_policy = EnumOrUnknown::from_i32(unit.get_restart_policy().clone() as i32);
            proto_unit.uid = unit.get_uid() as i32;
            proto_unit.gid = unit.get_gid() as i32;
            proto_unit.enabled = unit.is_enabled();
            proto_unit.process_probe_state = EnumOrUnknown::from_i32(unit.get_process_probe_state().clone() as i32);
            proto_unit.liveness_probe_state = EnumOrUnknown::from_i32(unit.get_liveness_probe_state().clone() as i32);
            proto_unit.state = EnumOrUnknown::from_i32(unit.get_state().clone() as i32);

            match unit.get_pid() {
                Some(pid) => proto_unit.pid = pid as i32,
                None => proto_unit.pid = -1,
            }

            match unit.get_uptime() {
                Some(uptime) => proto_unit.uptime = uptime.as_secs(),
                None => proto_unit.uptime = 0,
            }

            Ok(proto_unit)
        },
        Err(_) => {
            return Err("Failed to lock unit".to_string());
        },
    }
}