class Maenifold < Formula
  desc "Persistent Graph-of-Thoughts for AI agents and multi-agent systems"
  homepage "https://github.com/MSBrett/maenifold"
  version "1.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/MSBrett/maenifold/releases/download/v1.0.0/maenifold-osx-arm64.tar.gz"
      sha256 "8c745802820766a25f08b0d25bf03412ab6ec16398675cd5ef7dabcfc3ac89fa"
    else
      url "https://github.com/MSBrett/maenifold/releases/download/v1.0.0/maenifold-osx-x64.tar.gz"
      sha256 "f6c209cb7fe18bed87504547bb0a879267b448e3a4f75e6ed5282bda8eaffc83"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/MSBrett/maenifold/releases/download/v1.0.0/maenifold-linux-x64.tar.gz"
      sha256 "df68b6ec8c75a0f9df82e9b245b1390ac705f4ddfd8fbafebd517966bbc0118f"
    end
  end

  def install
    bin.install "Maenifold" => "maenifold"
  end

  test do
    # Test that the binary runs and returns valid output
    output = shell_output("#{bin}/maenifold --tool MemoryStatus --payload '{}'")
    assert_match(/memory root/i, output)
  end

  def caveats
    <<~EOS
      To use maenifold, set the memory root directory:
        export maenifold_ROOT=~/maenifold

      For MCP integration with Claude Code, add to ~/.claude/config.json:
        {
          "mcpServers": {
            "maenifold": {
              "command": "maenifold",
              "args": ["--mcp"],
              "env": {"maenifold_ROOT": "~/maenifold"}
            }
          }
        }

      Documentation: https://github.com/MSBrett/maenifold
    EOS
  end
end
