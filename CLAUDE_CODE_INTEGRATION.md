# Claude Code Integration

Stakpak now supports direct integration with Anthropic's Claude API, with **two authentication methods**:

1. **Claude Pro/Max OAuth** - Use your Claude subscription (FREE API usage included!)
2. **Anthropic API Key** - Use pay-per-use API keys from console.anthropic.com

## Quick Start

### Option 1: Claude Pro/Max OAuth (Recommended - FREE!)

Use your existing Claude Pro or Claude Max subscription:

```bash
# Login with your Claude Pro/Max account
stakpak auth login anthropic

# Follow the browser prompts to authenticate
# Then use stakpak normally - API usage is FREE with your subscription!
stakpak
```

### Option 2: Environment Variable (API Key)

For pay-per-use with an API key:

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
stakpak
```

### Option 3: Configuration Profile (API Key)

Add an Anthropic-powered profile to your `~/.stakpak/config.toml`:

```toml
[profiles.claude]
anthropic_api_key = "sk-ant-api03-..."
provider = "anthropic"
```

Then use it:

```bash
stakpak --profile claude
```

Or set it as default:

```bash
export STAKPAK_PROFILE=claude
stakpak
```

### Option 4: Configuration Profile (OAuth)

After running `stakpak auth login anthropic`, your config is automatically updated:

```toml
[profiles.default]
provider = "anthropic"

[profiles.default.anthropic_oauth]
refresh_token = "<stored securely>"
access_token = "<auto-refreshed>"
expires = 1234567890000
```

### Option 5: Mixed Configuration

You can have both Stakpak and Anthropic profiles in your config:

```toml
[profiles.default]
api_key = "stkpk_api_..."
provider = "stakpak"

[profiles.claude]
anthropic_api_key = "sk-ant-api03-..."
provider = "anthropic"

[profiles.mixed]
# Both keys - will use Anthropic by default
api_key = "stkpk_api_..."
anthropic_api_key = "sk-ant-api03-..."
provider = "anthropic"
```

## Features

### Model Selection

Both Stakpak and Anthropic providers use the same model tier system:

- **Smart Mode** (default): Claude Sonnet 4 (via Anthropic) or Stakpak Smart tier
- **Eco Mode**: Claude Haiku 4 (via Anthropic) or Stakpak Eco tier

The model selection is transparent - your existing commands work the same way:

```bash
# Use smart model (default)
stakpak "analyze this codebase"

# Use eco model for faster, cheaper operations
stakpak --model eco "run tests"
```

### All CLI Features Supported

The Anthropic integration supports all core Stakpak CLI features:

✅ Interactive TUI mode
✅ Async mode (`--async`)
✅ Print mode (`--print`)
✅ MCP server mode
✅ ACP mode (for editors)
✅ Tool calling and execution
✅ Streaming responses
✅ Privacy mode
✅ File operations with backups

### Hybrid Workflows

Some features still require the Stakpak backend:

- **Rulebooks** - Custom organizational policies
- **Agent sessions** - Session tracking and analytics
- **Code indexing** - Semantic search for infrastructure code
- **Memory/context** - Long-term memory across sessions

When using Anthropic mode, these features will show helpful error messages if accessed.

## Configuration Reference

### Profile Options

```toml
[profiles.myprofile]
# Stakpak API configuration
api_endpoint = "https://apiv2.stakpak.dev"  # Optional, defaults to official endpoint
api_key = "stkpk_api_..."                    # Your Stakpak API key

# Anthropic/Claude Code configuration
anthropic_api_key = "sk-ant-api03-..."       # Your Anthropic API key
provider = "anthropic"                        # Force provider: "anthropic" or "stakpak"

# Tool restrictions
allowed_tools = ["view", "run_command"]      # Limit which tools can be used
auto_approve = ["view"]                      # Auto-approve these tools

# Rulebooks (Stakpak only)
[profiles.myprofile.rulebooks]
include = ["stakpak://*/deployment-*"]
include_tags = ["production"]

# Warden/Security (Stakpak only)
[profiles.myprofile.warden]
enabled = true
volumes = [
  "~/.stakpak/config.toml:/config:ro",
  "./:/agent:ro"
]
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (overrides config) |
| `STAKPAK_API_KEY` | Stakpak API key (overrides config) |
| `STAKPAK_API_ENDPOINT` | Custom Stakpak endpoint |
| `STAKPAK_PROFILE` | Profile to use (default: "default") |

## Authentication Methods

### Method 1: Claude Pro/Max OAuth (FREE API Access!)

**Benefits:**
- ✅ Free API usage included with your Claude Pro/Max subscription
- ✅ No per-token charges
- ✅ Same models as API (Claude Sonnet 4, Haiku 4)
- ✅ Automatic token refresh

**How to use:**

```bash
# Login once
stakpak auth login anthropic

# Your browser will open to claude.ai
# Login with your Claude Pro/Max account
# Copy the authorization code shown
# Paste it into the terminal

# Done! Now use stakpak for free
stakpak "help me refactor this code"
```

**Logout:**

```bash
stakpak auth logout anthropic
```

### Method 2: Anthropic API Key (Pay-per-use)

**When to use:**
- You don't have Claude Pro/Max subscription
- You need programmatic API access
- You're building commercial applications

**How to get an API key:**

1. Sign up at [console.anthropic.com](https://console.anthropic.com)
2. Navigate to API Keys section
3. Create a new API key
4. Use it:

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
stakpak
```

Or add to config:

```toml
[profiles.default]
anthropic_api_key = "sk-ant-api03-..."
provider = "anthropic"
```

### Method 3: Stakpak Backend

**When to use:**
- You want access to rulebooks and enterprise features
- You need session tracking and analytics
- You want semantic code search for infrastructure

```bash
stakpak login --api-key <your-stakpak-key>
```

Or visit [stakpak.dev/generate-api-keys](https://stakpak.dev/generate-api-keys)

## Examples

### Use Claude Pro/Max subscription (FREE!)

```bash
# Login once
stakpak auth login anthropic

# Use for free forever (included in your subscription)
stakpak "review my code"
stakpak --async "refactor src/ to use async/await"
```

### Use API key for a single session

```bash
ANTHROPIC_API_KEY="sk-ant-..." stakpak
```

### Switch between providers

```bash
# Use Anthropic OAuth (free with subscription)
stakpak --profile default "review my code"

# Use Stakpak backend (for enterprise features)
stakpak --profile stakpak "review my code"
```

### Use Claude Code for async tasks

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
stakpak --async "refactor src/main.rs to use async/await"
```

### MCP server with Claude Code

```bash
# Start MCP server powered by Claude
export ANTHROPIC_API_KEY="sk-ant-..."
stakpak mcp
```

Then configure in Claude Desktop:

```json
{
  "mcpServers": {
    "stakpak": {
      "command": "stakpak",
      "args": ["mcp"],
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-..."
      }
    }
  }
}
```

## Provider Selection Logic

Stakpak automatically selects the provider based on available configuration:

1. If `provider` is explicitly set in the profile, use that
2. Otherwise, if `ANTHROPIC_API_KEY` or `anthropic_api_key` is set, use Anthropic
3. Otherwise, use Stakpak backend

This means you can seamlessly switch between providers without changing your config by just setting/unsetting environment variables.

## Pricing

### Claude Pro/Max OAuth (Recommended)

**FREE!** 🎉

When you login with `stakpak auth login anthropic` using your Claude Pro or Max subscription:
- ✅ **$0 per API call** - Included in your subscription
- ✅ All models available (Sonnet 4, Haiku 4)
- ✅ Same quality as paid API
- ✅ No usage limits beyond your subscription tier

This is the same authentication used by Claude Code and other official clients.

### Anthropic API Key (Pay-per-use)

When using an API key from console.anthropic.com:

- **Claude Sonnet 4**: $3/1M input tokens, $15/1M output tokens
- **Claude Haiku 4**: $1/1M input tokens, $5/1M output tokens

See current pricing at [anthropic.com/pricing](https://www.anthropic.com/pricing)

### Stakpak Backend

Stakpak offers competitive pricing with additional features:

- **Smart Model**: $3-6/1M input, $15-22.5/1M output (includes caching, sessions, rulebooks)
- **Eco Model**: $1/1M input, $5/1M output

## Troubleshooting

### "Anthropic authentication not found" error

You need to either login with OAuth or set an API key:

**Option 1: OAuth (FREE with Claude Pro/Max):**
```bash
stakpak auth login anthropic
```

**Option 2: API Key:**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Or add it to your profile:
```toml
[profiles.default]
anthropic_api_key = "sk-ant-..."
```

### "Token refresh failed" error

Your OAuth tokens may have expired. Re-login:

```bash
stakpak auth logout anthropic
stakpak auth login anthropic
```

### Authorization code format error

When pasting the authorization code, make sure to include both parts:
- Format: `<code>#<state>`
- Example: `abc123def456#xyz789`

Copy the entire string shown in your browser after authorization.

### "This operation requires a Stakpak API key" error

Some features (rulebooks, sessions, code indexing) require the Stakpak backend. You can configure both keys:

```toml
[profiles.hybrid]
api_key = "stkpk_api_..."           # For Stakpak features
anthropic_api_key = "sk-ant-..."    # For LLM calls
provider = "anthropic"               # Use Anthropic for inference
```

### Switching profiles

```bash
# List available profiles
stakpak config show

# Use a specific profile
stakpak --profile claude

# Or set as default
export STAKPAK_PROFILE=claude
```

## Support

- **Issues**: [github.com/stakpak/stakpak/issues](https://github.com/stakpak/stakpak/issues)
- **Documentation**: [docs.stakpak.dev](https://docs.stakpak.dev)
- **Discord**: [stakpak.dev/discord](https://stakpak.dev/discord)
