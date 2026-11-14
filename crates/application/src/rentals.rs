//! Rental use cases. The store preserves atomic revisions and idempotent dues.
use patrimoine_domain::*;

pub trait RentalService {
    fn list_rentals_at(&self, knowledge_sequence: Option<u64>) -> Result<RentalSnapshot>;
    fn save_lease(&mut self, input: SaveLeaseInput) -> Result<Lease>;
    fn save_lease_terms(&mut self, input: SaveLeaseTermsInput) -> Result<LeaseTerms>;
    fn save_rent_due(&mut self, input: SaveRentDueInput) -> Result<RentDue>;
    fn save_rent_payment(&mut self, input: SaveRentPaymentInput) -> Result<RentPayment>;
    fn refund_rent_payment(&mut self, input: RefundRentPaymentInput) -> Result<RentPayment>;
    fn save_rent_allocation(&mut self, input: SaveRentAllocationInput) -> Result<RentAllocation>;
    fn save_rental_expense(&mut self, input: SaveRentalExpenseInput) -> Result<RentalExpense>;
    fn generate_rent_dues(&mut self, input: GenerateRentDuesInput) -> Result<RentGenerationResult>;
    fn get_rental_report(&self, request: RentalReportRequest) -> Result<RentalReport>;
}
