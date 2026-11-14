//! Treasury commands preserve one source for every actual cash movement.
use patrimoine_domain::*;

pub trait TreasuryService {
    fn list_treasury_at(&self, knowledge_sequence: Option<u64>) -> Result<TreasurySnapshot>;
    fn save_treasury_account(&mut self, input: SaveTreasuryAccountInput)
        -> Result<TreasuryAccount>;
    fn save_treasury_transaction(
        &mut self,
        input: SaveTreasuryTransactionInput,
    ) -> Result<TreasuryTransaction>;
    fn save_account_reconciliation(
        &mut self,
        input: SaveAccountReconciliationInput,
    ) -> Result<AccountReconciliation>;
    fn save_cash_link(&mut self, input: SaveCashLinkInput) -> Result<CashLink>;
    fn save_deposit_movement(&mut self, input: SaveDepositMovementInput)
        -> Result<DepositMovement>;
    fn post_rent_payment(&mut self, input: PostRentPaymentInput) -> Result<RentPayment>;
    fn post_rental_expense(&mut self, input: PostRentalExpenseInput) -> Result<RentalExpense>;
    fn post_deposit(&mut self, input: PostDepositInput) -> Result<DepositMovement>;
    fn post_debt_movement(&mut self, input: PostDebtMovementInput) -> Result<DebtMovement>;
    fn get_treasury_report(&self, request: TreasuryReportRequest) -> Result<TreasuryReport>;
}
