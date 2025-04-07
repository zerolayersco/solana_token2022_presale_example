# Solana Token + Presale Example

This project demonstrates a simple token creation and presale system on Solana using the Anchor framework. It consists of two programs:

1. **Token Program**: For creating and managing custom SPL tokens
2. **Presale Program**: For running a token presale/fundraising event

## How It Works

### Token Program

The token program uses the Token-2022 standard and allows you to:

- Create a new token with custom name, symbol, and metadata
- Mint additional tokens 
- Burn tokens
- Transfer tokens between accounts
- Approve delegate spending

### Presale Program

The presale program allows you to run a presale event where:

- Users contribute SOL to purchase tokens at a fixed price
- Administrators can set minimum/maximum contribution amounts
- Soft cap and hard cap determine success criteria
- Tokens are distributed after the presale ends (if successful)
- Refunds can be issued if the presale fails

