use btleplug::api::ScanFilter;

pub(crate) fn vesc_tool_scan_filter() -> ScanFilter {
    ScanFilter::default()
}

#[cfg(test)]
mod tests {
    use super::vesc_tool_scan_filter;

    #[test]
    fn does_not_filter_by_service_uuid() {
        assert!(vesc_tool_scan_filter().services.is_empty());
    }
}
