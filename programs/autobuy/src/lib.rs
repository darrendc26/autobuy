use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

pub use instructions::*;
declare_id!("57rxgeNQuG6ypJyVGAfJMTpQX7cJESGdcLJyRCRhc9kN");

#[program]
pub mod autobuy {
    use super::*;

    pub fn make_delegate(ctx: Context<MakeDelegate>, amount: u64) -> Result<()> {
        make_delegate_handler(ctx, amount)
    }
}

#[derive(Accounts)]
pub struct Initialize {}
