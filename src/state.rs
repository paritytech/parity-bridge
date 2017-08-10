use futures::Future;

pub enum WithdrawRelay {
	WaitForNextLog,
	RelayTransaction,	
}

pub enum WithdrawConfirm {
	WaitForNextLog,
	ConfirmTransaction,
}

pub enum DepositRelay {
	WaitForNextLog,
	RelayTransaction,
}

pub enum DepositConfirm {
	WaitForNextLog,
	ConfirmDeposit,
}
