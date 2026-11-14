use patrimoine_domain::*;
use std::path::Path;

pub trait PortabilityService {
    fn export_csv_archive(&mut self, destination: &Path) -> Result<ExportSummary>;
    fn export_xlsx(&mut self, destination: &Path) -> Result<ExportSummary>;
    fn inspect_csv_import(&mut self, archive: &Path) -> Result<RestoreInspection>;
}
