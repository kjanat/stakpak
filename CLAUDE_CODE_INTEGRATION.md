# Claude Code Integration

Stakpak now supports direct integration with Anthropic's Claude API, allowing you to use your Claude Code subscription or Anthropic API key directly without going through Stakpak's backend.

## Quick Start

### Option 1: Environment Variable (Recommended)

Simply set your Anthropic API key as an environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
stakpak
```

Stakpak will automatically detect the Anthropic API key and use it instead of the Stakpak backend.

### Option 2: Configuration Profile

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

### Option 3: Mixed Configuration

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

## Getting Your API Key

### Anthropic API Key

1. Sign up at [console.anthropic.com](https://console.anthropic.com)
2. Navigate to API Keys section
3. Create a new API key
4. Set it as `ANTHROPIC_API_KEY` environment variable or in your config

### Stakpak API Key

```bash
stakpak login
```

Or visit [stakpak.dev/generate-api-keys](https://stakpak.dev/generate-api-keys)

## Examples

### Use Claude Code for a single session

```bash
ANTHROPIC_API_KEY="sk-ant-..." stakpak
```

### Switch between providers

```bash
# Use Anthropic
stakpak --profile claude "review my code"

# Use Stakpak backend
stakpak --profile default "review my code"
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

### Anthropic Direct

When using Anthropic directly, you pay Anthropic's standard API rates:

- **Claude Sonnet 4**: $3/1M input tokens, $15/1M output tokens
- **Claude Haiku 4**: $1/1M input tokens, $5/1M output tokens

See current pricing at [anthropic.com/pricing](https://www.anthropic.com/pricing)

### Stakpak Backend

Stakpak offers competitive pricing with additional features:

- **Smart Model**: $3-6/1M input, $15-22.5/1M output (includes caching, sessions, rulebooks)
- **Eco Model**: $1/1M input, $5/1M output

## Troubleshooting

### "Anthropic client not configured" error

Make sure you've set your API key:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Or add it to your profile:
```toml
[profiles.default]
anthropic_api_key = "sk-ant-..."
```

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
