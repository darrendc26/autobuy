use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Authority {
    pub user: Pubkey,
    pub bump: u8,
}
