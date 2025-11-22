use crate::state::authority::Authority;
use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Token, TokenAccount};

#[derive(Accounts)]
pub struct MakeDelegate<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(init,
        payer = user,
        space = 8 + Authority::INIT_SPACE,
        seeds = [b"authority".as_ref()],
        bump
    )]
    pub authority: Account<'info, Authority>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn make_delegate_handler(ctx: Context<MakeDelegate>, amount: u64) -> Result<()> {
    let authority = &mut ctx.accounts.authority;
    authority.user = ctx.accounts.user.key();
    authority.bump = ctx.bumps.authority;
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = token::Approve {
        to: ctx.accounts.user_token_account.to_account_info(),
        delegate: ctx.accounts.authority.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token::approve(cpi_ctx, amount)?;
    Ok(())
}
