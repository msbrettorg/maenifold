# Placeholders (VERSION_PLACEHOLDER, *_SHA_PLACEHOLDER) are substituted by the
# GitHub Actions release workflow (.github/workflows/release.yml), which copies
# this template into the homebrew-tap repo, replacing VERSION_PLACEHOLDER with
# the git tag (e.g. "1.0.3") and each SHA_PLACEHOLDER with the sha256 of the
# corresponding release archive.
class Maenifold < Formula
  desc "Context engineering infrastructure for AI agents"
  homepage "https://github.com/msbrettorg/maenifold"
  version "VERSION_PLACEHOLDER"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/msbrettorg/maenifold/releases/download/v#{version}/maenifold-osx-arm64.zip"
      sha256 "OSX_ARM64_SHA_PLACEHOLDER"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/msbrettorg/maenifold/releases/download/v#{version}/maenifold-linux-x64.zip"
      sha256 "LINUX_X64_SHA_PLACEHOLDER"
    end
  end

  def install
    # Single-file binary + assets directory. Binary uses AppContext.BaseDirectory
    # to find assets/, so both must live in the same directory.
    libexec.install "maenifold", "assets"
    bin.install_symlink libexec/"maenifold"
  end

  def caveats
    <<~EOS
      maenifold has been installed. To use with Claude Code, install the plugin:
        claude plugin add https://github.com/msbrettorg/maenifold/tree/main/integrations/claude-code/plugin-maenifold
    EOS
  end

  test do
    assert_match "maenifold", shell_output("#{bin}/maenifold --help")
  end
end
