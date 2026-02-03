#![allow(unused_imports)]

use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::Escrow;

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: we just need maker address
    #[account(mut)]
    pub maker: UncheckedAccount<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_b: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_b: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {

    // Send B to maker
    pub fn deposit(&mut self) -> Result<()> {
        require!(self.taker_ata_b.amount >= self.escrow.receive, ErrorCode::InsufficientFunds);

        // Transfer from taker to maker using escrow receive value
        let transfer_accounts = TransferChecked{
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };

        let cpi_context = CpiContext::new(
            self.token_program.to_account_info(),
            transfer_accounts,
        );
        transfer_checked(cpi_context, self.escrow.receive, self.mint_b.decimals)
    }

    // Send A to taker
    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        // WITHDRAW
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            self.escrow.maker.as_ref(),
            &self.escrow.seed.to_le_bytes(),
            &[self.escrow.bump],
        ]];

        // Withdraw token A for taker
        let transfer_accounts_vault_tx_taker = TransferChecked{
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let cpi_context_token_out = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            transfer_accounts_vault_tx_taker,
            signer_seeds
        );
        transfer_checked(cpi_context_token_out, self.vault.amount, self.mint_a.decimals)?;
        

        // Close vault with escrow sign CPI to close account
        let cpi_close_account = CloseAccount { 
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_close_account,
            signer_seeds
        );

        close_account(cpi_context)
    }
}

#[error_code]
pub enum ErrorCode{
    #[msg("Taker have insufficient token")]
    InsufficientFunds,
}