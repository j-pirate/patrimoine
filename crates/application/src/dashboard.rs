//! Read-only dashboards and explicitly filtered reports, distinct from backups.
use patrimoine_domain::{DashboardReport, DashboardRequest, Result};
use std::path::Path;

pub trait DashboardService {
    fn get_dashboard(&self, request: DashboardRequest) -> Result<DashboardReport>;
    fn export_dashboard_report(
        &mut self,
        request: DashboardRequest,
        destination: &Path,
    ) -> Result<()>;
}
