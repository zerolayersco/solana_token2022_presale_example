use anchor_lang::prelude::*;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_spl::token_interface::{self, InitializeMint2, MintTo, Burn, TransferChecked, Approve};
use anchor_spl::associated_token::{self, Create};

declare_id!("your_program_pubkey_in_base58");

#[program]
pub mod token {
    use super::*;

    pub fn initialize_token(
        ctx: Context<InitializeToken>,
        name: String,
        symbol: String,
        uri: String,
        decimals: u8,
        supply: u64,
    ) -> Result<()> {
        msg!("Initializing ZL Token (zerolayers) with Token Interface");
        
        let token_state = &mut ctx.accounts.token_state;
        token_state.authority = ctx.accounts.authority.key();
        token_state.mint = *ctx.accounts.mint.key;
        token_state.name = name;
        token_state.symbol = symbol;
        token_state.uri = uri;
        token_state.decimals = decimals;
        token_state.total_supply = supply;
        
        let cpi_accounts_init = InitializeMint2 {
            mint: ctx.accounts.mint.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx_init = CpiContext::new(cpi_program.clone(), cpi_accounts_init);
        
        token_interface::initialize_mint2(
            cpi_ctx_init,
            token_state.decimals,
            &ctx.accounts.authority.key(),
            Some(&ctx.accounts.authority.key()),
        )?;
        
        msg!("Mint initialized with {} decimals", decimals);
        
        let cpi_accounts_ata = Create {
            payer: ctx.accounts.authority.to_account_info(),
            associated_token: ctx.accounts.token_account.clone(),
            authority: ctx.accounts.authority.to_account_info(),
            mint: ctx.accounts.mint.clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.clone(),
        };
        
        let cpi_program_ata = ctx.accounts.associated_token_program.clone();
        let cpi_ctx_ata = CpiContext::new(cpi_program_ata, cpi_accounts_ata);
        
        associated_token::create(cpi_ctx_ata)?;
        
        msg!("Associated Token Account created");
        
        let cpi_accounts_mint = MintTo {
            mint: ctx.accounts.mint.clone(),
            to: ctx.accounts.token_account.clone(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        
        let cpi_ctx_mint = CpiContext::new(cpi_program, cpi_accounts_mint);
        
        token_interface::mint_to(cpi_ctx_mint, token_state.total_supply)?;
        
        msg!("Successfully minted {} tokens", token_state.total_supply);
        
        Ok(())
    }

    pub fn mint_tokens(
        ctx: Context<MintTokens>,
        amount: u64,
    ) -> Result<()> {
        let token_state = &ctx.accounts.token_state;
        
        require!(
            ctx.accounts.authority.key() == token_state.authority,
            MyError::UnauthorizedMintAuthority
        );

        let token_state = ctx.accounts.token_state.to_account_info();
        let mut token_state_data = token_state.try_borrow_mut_data()?;
        let mut token_state_account = TokenState::try_deserialize(&mut &token_state_data[..])?;
        token_state_account.total_supply = token_state_account.total_supply.checked_add(amount).ok_or(MyError::OverflowError)?;
        token_state_account.try_serialize(&mut *token_state_data)?;

        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.clone(),
            to: ctx.accounts.token_account.clone(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token_interface::mint_to(cpi_ctx, amount)?;
        
        msg!("Successfully minted {} additional tokens", amount);
        
        Ok(())
    }

    pub fn burn_tokens(
        ctx: Context<BurnTokens>,
        amount: u64,
    ) -> Result<()> {
        let token_state = &ctx.accounts.token_state;
        
        require!(
            ctx.accounts.owner.key() == token_state.authority,
            MyError::UnauthorizedBurnAuthority
        );

        let token_state = ctx.accounts.token_state.to_account_info();
        let mut token_state_data = token_state.try_borrow_mut_data()?;
        let mut token_state_account = TokenState::try_deserialize(&mut &token_state_data[..])?;
        token_state_account.total_supply = token_state_account.total_supply.checked_sub(amount).ok_or(MyError::UnderflowError)?;
        token_state_account.try_serialize(&mut *token_state_data)?;

        let cpi_accounts = Burn {
            mint: ctx.accounts.mint.clone(),
            from: ctx.accounts.token_account.clone(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token_interface::burn(cpi_ctx, amount)?;
        
        msg!("Successfully burned {} tokens", amount);
        
        Ok(())
    }

    pub fn transfer_tokens(
        ctx: Context<TransferTokens>,
        amount: u64,
    ) -> Result<()> {
        msg!("Transferring {} tokens", amount);
        
        let token_state = &ctx.accounts.token_state;
        let decimals = token_state.decimals;
        
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.from.clone(),
            to: ctx.accounts.to.clone(),
            authority: ctx.accounts.owner.to_account_info(),
            mint: ctx.accounts.mint.clone(),
        };
        
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token_interface::transfer_checked(cpi_ctx, amount, decimals)?;
        
        msg!("Successfully transferred {} tokens", amount);
        
        Ok(())
    }

    pub fn approve_tokens(
        ctx: Context<ApproveTokens>,
        amount: u64,
    ) -> Result<()> {
        msg!("Approving {} tokens for delegate", amount);
        
        let cpi_accounts = Approve {
            to: ctx.accounts.token_account.clone(),
            delegate: ctx.accounts.delegate.clone(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token_interface::approve(cpi_ctx, amount)?;
        
        msg!("Successfully approved {} tokens for delegate", amount);
        
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct TokenState {
    pub authority: Pubkey,    // 32 bytes
    pub mint: Pubkey,         // 32 bytes
    pub name: String,         // varies
    pub symbol: String,       // varies
    pub uri: String,          // varies
    pub decimals: u8,         // 1 byte
    pub total_supply: u64,    // 8 bytes
}

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 4 + 50 + 4 + 10 + 4 + 200 + 1 + 8
    )]
    pub token_state: Account<'info, TokenState>,
    
    /// CHECK: This is the mint account that is initialized in the instruction
    #[account(
        init,
        payer = authority,
        space = 82,  // Minimum space for a mint account
        owner = token_program.key()
    )]
    pub mint: AccountInfo<'info>,
    
    /// CHECK: This is the token account that is initialized in the instruction
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    
    /// CHECK: This is the token program ID
    pub token_program: AccountInfo<'info>,
    
    /// CHECK: This is the associated token program ID
    pub associated_token_program: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(mut)]
    pub token_state: Account<'info, TokenState>,
    
    /// CHECK: This is the mint account
    #[account(
        mut,
        constraint = token_state.mint == *mint.key
    )]
    pub mint: AccountInfo<'info>,
    
    /// CHECK: This is the token account that will receive the minted tokens
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    
    /// CHECK: This is the token program ID
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    #[account(mut)]
    pub token_state: Account<'info, TokenState>,
    
    /// CHECK: This is the mint account
    #[account(
        mut,
        constraint = token_state.mint == *mint.key
    )]
    pub mint: AccountInfo<'info>,
    
    /// CHECK: This is the token account that the tokens will be burned from
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    
    /// CHECK: This is the token program ID
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: This is the sender's token account
    #[account(mut)]
    pub from: AccountInfo<'info>,
    
    /// CHECK: This is the recipient's token account
    #[account(mut)]
    pub to: AccountInfo<'info>,
    
    /// CHECK: This is the mint account
    pub mint: AccountInfo<'info>,
    
    pub token_state: Account<'info, TokenState>,
    
    /// CHECK: This is the token program ID
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ApproveTokens<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: This is the token account
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    
    /// CHECK: This is the delegate that will be approved to spend tokens
    pub delegate: AccountInfo<'info>,
    
    /// CHECK: This is the token program ID
    pub token_program: AccountInfo<'info>,
}

#[error_code]
pub enum MyError {
    #[msg("Only the mint authority can mint new tokens")]
    UnauthorizedMintAuthority,
    #[msg("Only the owner can burn tokens")]
    UnauthorizedBurnAuthority,
    #[msg("Arithmetic overflow")]
    OverflowError,
    #[msg("Arithmetic underflow")]
    UnderflowError,
}
