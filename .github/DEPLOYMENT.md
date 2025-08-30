# Deployment Configuration

## GitHub Secrets Required

The following secrets need to be configured in your GitHub repository settings:

### Required Secrets

1. **`DATABASE_URL`** - PostgreSQL connection string for SQLx compilation
   - Format: `postgres://user:password@host:port/database`
   - Used during build to verify SQL queries

2. **`COOLIFY_API_TOKEN`** - API token for triggering Coolify deployments
   - Get this from your Coolify dashboard
   - Required for automatic deployments after successful builds

## Setting up Secrets

1. Go to your repository on GitHub
2. Navigate to Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Add each secret with the appropriate value

## Deployment Flow

1. Push to `main` branch triggers the workflow
2. Backend and frontend are built in parallel
3. Docker image is built and pushed to GitHub Container Registry
4. Coolify deployment is triggered via API
5. Coolify pulls the new image and deploys it

## Coolify Configuration

- **Deployment URL**: `https://cf.avolut.com/api/v1/deploy`
- **UUID**: `k080sksw0g0o0wgkw4440084`
- **Force Deploy**: `false` (uses smart deployment)

## Manual Deployment

To manually trigger a deployment:

```bash
curl -X GET \
  -H "Authorization: Bearer YOUR_COOLIFY_API_TOKEN" \
  "https://cf.avolut.com/api/v1/deploy?uuid=k080sksw0g0o0wgkw4440084&force=false"
```

## Troubleshooting

- Check GitHub Actions logs for build failures
- Verify all secrets are properly configured
- Ensure Coolify API token has deployment permissions
- Check Coolify dashboard for deployment status