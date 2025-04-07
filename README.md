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

The presale program allows you to run a fundraising event where:

- Users contribute SOL to purchase tokens at a fixed price
- Administrators can set minimum/maximum contribution amounts
- Soft cap and hard cap determine success criteria
- Tokens are distributed after the presale ends (if successful)
- Refunds can be issued if the presale fails

## Step-by-Step Guide

### Setting Up the Token

1. **Initialize Token**: 
   ```bash
   # Create a new token with name, symbol, and metadata
   anchor run initialize-token -- --name "My Token" --symbol "MT" --decimals 9 --supply 1000000
   ```

2. **Get Token State**:
   ```bash
   # View token details
   anchor run token-state
   ```

### Setting Up the Presale

3. **Initialize Presale**:
   ```bash
   # Create a new presale with parameters
   anchor run initialize-presale -- --soft-cap 50 --hard-cap 100 --token-price 0.5 --start-time 1714504800 --end-time 1717183200 --min-contribution 0.1
   ```

4. **Check Presale State**:
   ```bash
   # View presale details
   anchor run presale-state
   ```

### User Participation

5. **Contributing**:
   ```bash
   # Contribute SOL to the presale
   anchor run contribute -- --amount 1.5
   ```

6. **Check Your Contribution**:
   ```bash
   # View your contribution amount and token entitlement
   anchor run get-contribution
   ```

### Completing the Presale

7. **Enable Claims** (Admin only, if soft cap reached):
   ```bash
   # Allow users to claim tokens
   anchor run enable-claims
   ```

8. **Enable Refunds** (Admin only, if presale failed):
   ```bash
   # Allow users to get refunds
   anchor run enable-refunds
   ```

9. **Claim Tokens** (After claims enabled):
   ```bash
   # Get your tokens
   anchor run claim-tokens
   ```

10. **Get Refund** (If refunds enabled):
    ```bash
    # Get your SOL back
    anchor run refund
    ```

11. **Finalize Presale** (Admin only, after successful presale):
    ```bash
    # Collect raised funds
    anchor run finalize-presale
    ```

## Important Notes

- The presale will only succeed if it reaches at least the soft cap
- Token claims are only enabled if the presale reaches the soft cap
- Refunds are issued if the presale fails to reach the soft cap
- All times are in Unix timestamp format
- Contributions are accepted only between start and end time
- Each user must contribute at least the minimum amount
- Total contributions cannot exceed the hard cap

## Security Considerations

- The admin has significant control - make sure you trust the presale creator
- Always verify contract addresses before interacting
- Double-check token prices and contribution amounts
- Verify presale parameters before contributing 