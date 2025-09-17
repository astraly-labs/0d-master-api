# 0D Finance Vault Aggregator API

## Project Overview

This is a master API that aggregates data from multiple vault APIs and blockchain-indexed user data to provide a unified interface for the 0D Finance platform. The API serves as an orchestrator that combines vault metadata from the database, performance data from individual vault APIs, and user transaction data indexed from the blockchain.

## Architecture

### System Components

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Frontend      │────▶│  Master API      │────▶│  Vault APIs     │
│                 │     │   (Axum/Rust)    │     │  (Individual)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                               │                          
                               ▼                          
                        ┌──────────────────┐              
                        │    PostgreSQL    │◀────────────┐
                        │                  │              │
                        └──────────────────┘              │
                                                    ┌──────────────────┐
                                                    │  Indexer Service │
                                                    │                  │
                                                    └──────────────────┘
                                                           ▲
                                                           │
                                                    ┌──────────────────┐
                                                    │   Blockchain     │
                                                    │   (Starknet)     │
                                                    └──────────────────┘
```

### Data Flow

1. **Blockchain → Indexer**: On-chain events are indexed and stored in user_transactions
2. **Indexer → Database**: User positions are updated based on transactions
3. **API Request**: Master API receives request for vault/user data
4. **Data Aggregation**:
   - Fetch vault metadata from DB
   - Call individual vault API for performance data
   - Query user data from DB if needed
   - Calculate KPIs if needed (or use cached)
5. **Response**: Merge all data and return formatted response

## Current Technical Stack

- **API Framework**: Axum (Rust)
- **Database**: PostgreSQL
- **Blockchain**: Starknet (testnet for dev, mainnet for prod)
- **Current Performance**: 
  - 10 RPS volume
  - 5-10 consumers
  - 95% SLA with P99<1s latency

## API Specification

### Base URLs
- Production: `https://api.0d.finance/v1` (Starknet mainnet)
- Development: `https://dev-api.0d.finance/v1` (Starknet testnet)

### Main Endpoints

#### Vault Endpoints
- `GET /vaults` - List all vaults with basic info
- `GET /vaults/{vault_id}` - Get vault metadata
- `GET /vaults/{vault_id}/stats` - Get vault statistics
- `GET /vaults/{vault_id}/timeseries` - Historical performance data
- `GET /vaults/{vault_id}/kpis` - Performance KPIs
- `GET /vaults/{vault_id}/apr/summary` - APR summary
- `GET /vaults/{vault_id}/composition` - Current vault composition

#### User Endpoints
- `GET /users/{address}` - User profile
- `GET /users/{address}/vaults/{vault_id}/summary` - User position summary
- `GET /users/{address}/vaults/{vault_id}/historical` - User performance history
- `GET /users/{address}/vaults/{vault_id}/kpis` - User KPIs
- `GET /users/{address}/vaults/{vault_id}/transactions` - Transaction history

## Database Schema

### Core Tables

#### vaults
- Stores vault metadata, configuration, and API endpoints
- Each vault has its own API endpoint for fetching performance data
- Status: live/paused/retired

#### users
- User registry indexed from blockchain
- Stores wallet addresses and preferences

#### user_positions
- Current user positions in vaults
- Updated by indexer based on blockchain events
- Tracks share_balance and cost_basis

#### user_transactions
- All user transactions indexed from blockchain
- Types: deposit/withdraw/fee/rebalance
- Includes tx_hash, block_number, amounts

#### user_kpis
- Calculated performance metrics
- Can be refreshed periodically or on-demand
- Includes PnL, Sharpe ratio, drawdown

#### indexer_state
- Tracks indexer progress per vault
- Monitors last processed block
- Error tracking and status

### Key Design Decisions

- **DECIMAL(36,18)**: For precise financial calculations (Ethereum standard)
- **String for amounts in API**: Avoid JavaScript number precision issues
- **JSONB metadata**: Flexible storage for chain-specific data
- **Separate KPIs table**: Allows caching expensive calculations
- **API endpoint in vault table**: Each vault has its own API for performance data

## Implementation Guidelines

### Data Aggregation Pattern

```rust
// Pseudo-code for endpoint implementation
async fn get_vault_summary(vault_id: String) -> Result<VaultSummary> {
    // 1. Fetch vault metadata from DB
    let vault_metadata = db.get_vault(vault_id)?;
    
    // 2. Call vault's individual API for performance data
    let performance_data = http_client
        .get(vault_metadata.api_endpoint)
        .await?;
    
    // 3. Merge data
    let summary = VaultSummary {
        ...vault_metadata,
        ...performance_data,
    };
    
    Ok(summary)
}
```

### Caching Strategy

- Use Redis for frequently accessed vault metadata
- Cache current TVL, APR values with short TTL (1-5 minutes)
- Implement Cache-Control headers for client-side caching
- Use ETags for efficient cache invalidation

### Error Handling

- Return consistent error format with code, message, request_id
- Use appropriate HTTP status codes
- Log errors with context for debugging
- Implement circuit breaker for vault API calls

### Security Considerations

- Implement rate limiting (already have IP rate limiting)
- Validate all input parameters
- Use parameterized queries to prevent SQL injection
- Implement request signing for vault-to-vault API calls
- CORS configuration for frontend domains

## Performance Optimizations

### Database
- Composite indexes for common query patterns
- Consider partitioning user_transactions by month
- Use materialized views for complex calculations
- Connection pooling with appropriate limits

### API
- Implement response compression
- Use async/await for concurrent vault API calls
- Batch requests where possible
- Implement pagination for large result sets

## Monitoring and Observability

### Metrics to Track
- Request latency (P50, P95, P99)
- Error rates by endpoint
- Vault API response times
- Database query performance
- Indexer lag (blocks behind)

### Logging
- Structured logging with correlation IDs
- Log levels: ERROR, WARN, INFO, DEBUG
- Include user_address and vault_id in context
- Rotate logs daily, retain for 30 days

## Development Workflow

### Environment Setup
```bash
# Database migrations
diesel migration run

# Environment variables
DATABASE_URL=postgresql://user:pass@localhost/vaults
REDIS_URL=redis://localhost:6379
ENVIRONMENT=development  # or production
```

### Testing Strategy
- Unit tests for business logic
- Integration tests for API endpoints
- Mock vault APIs for testing
- Load testing for performance validation

## Future Considerations

### Scaling (when needed)
- Add Kong API Gateway when:
  - Traffic exceeds 100-200 RPS
  - Number of consumers > 50
  - Need complex authentication (OAuth2)
  - Multiple microservices added

### Multi-chain Support
- Database schema already supports chain field
- Consider separate indexer per chain
- May need chain-specific transaction parsing

### Additional Features
- WebSocket support for real-time updates
- GraphQL endpoint for flexible queries
- Batch operations for multiple vaults
- Historical snapshots for backtesting

## Common Issues and Solutions

### Issue: Vault API timeout
**Solution**: Implement circuit breaker, return cached data if available

### Issue: Indexer lag
**Solution**: Monitor block height, alert if > 10 blocks behind

### Issue: KPI calculation slow
**Solution**: Pre-calculate and cache, refresh async in background

### Issue: Database connection pool exhausted
**Solution**: Increase pool size, optimize queries, implement connection retry

## Contact and Resources

- OpenAPI Spec: `/docs/openapi.yaml`
- Vault API Documentation: Individual vault endpoints
- Indexer Documentation: [Internal docs]
- Monitoring Dashboard: [Grafana URL]

## Notes for Development

- All monetary values use DECIMAL(36,18) in DB
- Always return amounts as strings in API responses
- Validate vault_id exists before calling vault API
- Check indexer_state before returning user data
- Use UTC timestamps throughout the system
- Test with both testnet and mainnet configurations