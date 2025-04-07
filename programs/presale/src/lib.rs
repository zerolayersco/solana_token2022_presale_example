use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Token};

declare_id!("your_program_pubkey_in_base58");

#[program]
pub mod presale {
    use super::*;

    pub fn initialize_presale(
        ctx: Context<InitializePresale>,
        soft_cap: u64,
        hard_cap: u64,
        token_price: u64,
        start_time: i64,
        end_time: i64,
        min_contribution: u64,
    ) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        
        presale_state.authority = ctx.accounts.authority.key();
        presale_state.soft_cap = soft_cap;
        presale_state.hard_cap = hard_cap;
        presale_state.token_price = token_price;
        presale_state.start_time = start_time;
        presale_state.end_time = end_time;
        presale_state.min_contribution = min_contribution;
        presale_state.total_contributions = 0;
        presale_state.claims_enabled = false;
        presale_state.refunds_enabled = false;
        presale_state.finalized = false;
        presale_state.treasury = ctx.accounts.treasury.key();
        
        msg!("Presale initialized with soft cap: {}, hard cap: {}", soft_cap, hard_cap);
        msg!("Presale period: {} to {}", start_time, end_time);
        
        Ok(())
    }

    pub fn contribute(
        ctx: Context<Contribute>,
        amount: u64
    ) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        let clock = Clock::get()?;
        
        require!(
            clock.unix_timestamp >= presale_state.start_time,
            PresaleError::PresaleNotStarted
        );
        require!(
            clock.unix_timestamp <= presale_state.end_time,
            PresaleError::PresaleEnded
        );
        require!(!presale_state.finalized, PresaleError::PresaleFinalized);
        require!(
            !presale_state.refunds_enabled,
            PresaleError::RefundsEnabled
        );

        require!(
            amount >= presale_state.min_contribution,
            PresaleError::ContributionTooSmall
        );
        require!(
            presale_state.total_contributions + amount <= presale_state.hard_cap,
            PresaleError::HardCapExceeded
        );

        let user_contribution = &mut ctx.accounts.user_contribution;
        user_contribution.contributor = ctx.accounts.contributor.key();
        user_contribution.amount = user_contribution.amount.checked_add(amount)
            .ok_or(PresaleError::AmountOverflow)?;
        user_contribution.claimed = false;
        user_contribution.refunded = false;

        presale_state.total_contributions = presale_state.total_contributions
            .checked_add(amount)
            .ok_or(PresaleError::AmountOverflow)?;

        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.contributor.key(),
            &ctx.accounts.treasury.key(),
            amount,
        );
        
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.contributor.to_account_info(),
                ctx.accounts.treasury.to_account_info(),
            ],
        )?;

        msg!("Contribution of {} lamports received from {}", 
            amount, 
            ctx.accounts.contributor.key());
        msg!("Total contributions: {}", presale_state.total_contributions);

        Ok(())
    }

    pub fn get_contribution(ctx: Context<GetContribution>) -> Result<()> {
        let user_contribution = &ctx.accounts.user_contribution;
        let presale_state = &ctx.accounts.presale_state;
        
        msg!("Contributor: {}", user_contribution.contributor);
        msg!("Contribution amount: {} lamports", user_contribution.amount);
        msg!("Claimed: {}", user_contribution.claimed);
        msg!("Refunded: {}", user_contribution.refunded);
        
        if user_contribution.amount > 0 && presale_state.token_price > 0 {
            let token_amount = user_contribution.amount
                .checked_div(presale_state.token_price)
                .unwrap_or(0);
            msg!("Tokens entitled: {}", token_amount);
        }
        
        Ok(())
    }
    
    pub fn enable_claims(ctx: Context<AdminAction>) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        
        require!(
            ctx.accounts.authority.key() == presale_state.authority,
            PresaleError::Unauthorized
        );
        
        require!(
            presale_state.total_contributions >= presale_state.soft_cap,
            PresaleError::SoftCapNotReached
        );
        
        presale_state.claims_enabled = true;
        presale_state.refunds_enabled = false;
        
        msg!("Claims enabled for presale");
        
        Ok(())
    }
    
    pub fn claim_tokens(ctx: Context<ClaimTokens>) -> Result<()> {
        let presale_state = &ctx.accounts.presale_state;
        let user_contribution = &mut ctx.accounts.user_contribution;
        
        require!(!user_contribution.claimed, PresaleError::AlreadyClaimed);
        
        require!(presale_state.claims_enabled, PresaleError::ClaimsNotEnabled);
        
        require!(
            ctx.accounts.contributor.key() == user_contribution.contributor,
            PresaleError::Unauthorized
        );
        
        let token_amount = user_contribution.amount
            .checked_div(presale_state.token_price)
            .ok_or(PresaleError::AmountOverflow)?;
            
        require!(token_amount > 0, PresaleError::InsufficientTokenBalance);
        
        let treasury_token_account_bump = ctx.bumps.treasury_token_account;
        let seeds = &[b"treasury_token_account" as &[u8], &[treasury_token_account_bump]];
        let signer = &[&seeds[..]];
        
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.treasury_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.treasury_token_account.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        
        anchor_spl::token::transfer(cpi_ctx, token_amount)?;
        
        user_contribution.claimed = true;
        
        msg!("Claimed {} tokens for contributor {}", 
            token_amount, 
            user_contribution.contributor);
        
        Ok(())
    }
    
    pub fn enable_refunds(ctx: Context<AdminAction>) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        
        require!(
            ctx.accounts.authority.key() == presale_state.authority,
            PresaleError::Unauthorized
        );
        
        presale_state.refunds_enabled = true;
        presale_state.claims_enabled = false;
        
        msg!("Refunds enabled for presale");
        
        Ok(())
    }
    
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let presale_state = &ctx.accounts.presale_state;
        let user_contribution = &mut ctx.accounts.user_contribution;
        
        require!(!user_contribution.refunded, PresaleError::AlreadyRefunded);
        
        require!(presale_state.refunds_enabled, PresaleError::RefundsNotEnabled);
        
        require!(
            ctx.accounts.contributor.key() == user_contribution.contributor,
            PresaleError::Unauthorized
        );
        
        let refund_amount = user_contribution.amount;
        require!(refund_amount > 0, PresaleError::NoRefundAvailable);
        
        let treasury_bump = ctx.bumps.treasury;
        let seeds = &[b"treasury" as &[u8], &[treasury_bump]];
        let signer = &[&seeds[..]];
        
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.treasury.key(),
            &ctx.accounts.contributor.key(),
            refund_amount,
        );
        
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.treasury.to_account_info(),
                ctx.accounts.contributor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer,
        )?;
        
        user_contribution.refunded = true;
        
        msg!("Refunded {} lamports to contributor {}", 
            refund_amount, 
            user_contribution.contributor);
        
        Ok(())
    }
    
    pub fn finalize_presale(ctx: Context<FinalizePresale>) -> Result<()> {
        let presale_state = &mut ctx.accounts.presale_state;
        
        require!(
            ctx.accounts.authority.key() == presale_state.authority,
            PresaleError::Unauthorized
        );
        
        require!(
            presale_state.claims_enabled && !presale_state.refunds_enabled,
            PresaleError::CannotFinalize
        );
        
        require!(
            presale_state.total_contributions >= presale_state.soft_cap,
            PresaleError::SoftCapNotReached
        );
        
        let treasury_balance = **ctx.accounts.treasury.try_borrow_lamports()?;
        
        let treasury_bump = ctx.bumps.treasury;
        let seeds = &[b"treasury" as &[u8], &[treasury_bump]];
        let signer = &[&seeds[..]];
        
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.treasury.key(),
            &ctx.accounts.admin_wallet.key(),
            treasury_balance,
        );
        
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.treasury.to_account_info(),
                ctx.accounts.admin_wallet.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer,
        )?;
        
        presale_state.finalized = true;
        
        msg!("Presale finalized. {} lamports transferred to admin wallet {}", 
            treasury_balance, 
            ctx.accounts.admin_wallet.key());
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(
    soft_cap: u64,
    hard_cap: u64,
    token_price: u64,
    start_time: i64,
    end_time: i64,
    min_contribution: u64
)]

pub struct InitializePresale<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + PresaleState::SIZE,
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(
        seeds = [b"treasury"],
        bump
    )]
    /// CHECK: This is the PDA that will collect SOL from contributions
    pub treasury: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[account]
pub struct PresaleState {
    pub authority: Pubkey,        // Admin address
    pub soft_cap: u64,            // Minimum amount to raise in lamports
    pub hard_cap: u64,            // Maximum amount to raise in lamports
    pub token_price: u64,         // Price per token in lamports
    pub start_time: i64,          // Start timestamp
    pub end_time: i64,            // End timestamp
    pub min_contribution: u64,    // Minimum contribution amount in lamports
    pub total_contributions: u64, // Total amount raised in lamports
    pub claims_enabled: bool,     // Whether token claims are enabled
    pub refunds_enabled: bool,    // Whether refunds are enabled
    pub finalized: bool,          // Whether the presale has been finalized
    pub treasury: Pubkey,         // Treasury PDA
}

impl PresaleState {
    pub const SIZE: usize = 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 1 + 1 + 32; // Size in bytes
}

#[account]
pub struct UserContribution {
    pub contributor: Pubkey,      // User's wallet address
    pub amount: u64,              // Contribution amount in lamports
    pub claimed: bool,            // Whether user has claimed tokens
    pub refunded: bool,           // Whether user has received a refund
}

impl UserContribution {
    pub const SIZE: usize = 32 + 8 + 1 + 1; // Size in bytes
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(
        mut,
        seeds = [b"treasury"],
        bump
    )]
    /// CHECK: This is the PDA that collects SOL from contributions
    pub treasury: AccountInfo<'info>,
    
    #[account(
        init_if_needed,
        payer = contributor,
        space = 8 + UserContribution::SIZE,
        seeds = [b"user_contribution", contributor.key().as_ref()],
        bump
    )]
    pub user_contribution: Account<'info, UserContribution>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetContribution<'info> {
    /// CHECK: Read-only account, no need to validate
    pub contributor: AccountInfo<'info>,
    
    #[account(
        seeds = [b"user_contribution", contributor.key.as_ref()],
        bump
    )]
    pub user_contribution: Account<'info, UserContribution>,
    
    #[account(
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"presale_state"],
        bump,
        constraint = presale_state.authority == authority.key() @ PresaleError::Unauthorized
    )]
    pub presale_state: Account<'info, PresaleState>,
}

#[derive(Accounts)]
pub struct ClaimTokens<'info> {
    pub contributor: Signer<'info>,
    
    #[account(
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(
        mut,
        seeds = [b"user_contribution", contributor.key().as_ref()],
        bump,
        constraint = user_contribution.contributor == contributor.key() @ PresaleError::Unauthorized
    )]
    pub user_contribution: Account<'info, UserContribution>,
    
    #[account(
        mut,
        seeds = [b"treasury_token_account"],
        bump
    )]
    /// CHECK: This is the PDA that holds tokens for distribution
    pub treasury_token_account: AccountInfo<'info>,
    
    #[account(mut)]
    /// CHECK: This is the user's token account to receive tokens
    pub user_token_account: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,
    
    #[account(
        seeds = [b"presale_state"],
        bump
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(
        mut,
        seeds = [b"user_contribution", contributor.key().as_ref()],
        bump,
        constraint = user_contribution.contributor == contributor.key() @ PresaleError::Unauthorized
    )]
    pub user_contribution: Account<'info, UserContribution>,
    
    #[account(
        mut,
        seeds = [b"treasury"],
        bump
    )]
    /// CHECK: This is the PDA that holds the SOL contributions
    pub treasury: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizePresale<'info> {
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"presale_state"],
        bump,
        constraint = presale_state.authority == authority.key() @ PresaleError::Unauthorized
    )]
    pub presale_state: Account<'info, PresaleState>,
    
    #[account(
        mut,
        seeds = [b"treasury"],
        bump
    )]
    /// CHECK: This is the PDA that holds the SOL contributions
    pub treasury: AccountInfo<'info>,
    
    #[account(mut)]
    /// CHECK: This is the admin's wallet to receive funds
    pub admin_wallet: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum PresaleError {
    #[msg("Presale has not started yet")]
    PresaleNotStarted,
    #[msg("Presale has ended")]
    PresaleEnded,
    #[msg("Presale has been finalized")]
    PresaleFinalized,
    #[msg("Refunds are enabled, no more contributions accepted")]
    RefundsEnabled,
    #[msg("Contribution amount is below minimum")]
    ContributionTooSmall,
    #[msg("Hard cap would be exceeded")]
    HardCapExceeded,
    #[msg("Arithmetic overflow")]
    AmountOverflow,
    #[msg("Not enough tokens to claim")]
    InsufficientTokenBalance,
    #[msg("Claims are not enabled yet")]
    ClaimsNotEnabled,
    #[msg("Refunds are not enabled")]
    RefundsNotEnabled,
    #[msg("Only authority can perform this action")]
    Unauthorized,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("Already refunded")]
    AlreadyRefunded,
    #[msg("Soft cap not reached")]
    SoftCapNotReached,
    #[msg("No refund available")]
    NoRefundAvailable,
    #[msg("Cannot finalize presale")]
    CannotFinalize,
}
