# Placeholders (VERSION_PLACEHOLDER, *_SHA_PLACEHOLDER) are substituted by the
# release workflow: scripts/release.sh copies this template into the homebrew-tap
# repo, replacing VERSION_PLACEHOLDER with the git tag (e.g. "1.0.3") and each
# SHA_PLACEHOLDER with the sha256 of the corresponding release tarball.
class Maenifold < Formula
  desc "Context engineering infrastructure for AI agents"
  homepage "https://github.com/msbrettorg/maenifold"
  version "VERSION_PLACEHOLDER"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/msbrettorg/maenifold/releases/download/v#{version}/maenifold-osx-arm64.tar.gz"
      sha256 "OSX_ARM64_SHA_PLACEHOLDER"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/msbrettorg/maenifold/releases/download/v#{version}/maenifold-linux-x64.tar.gz"
      sha256 "LINUX_X64_SHA_PLACEHOLDER"
    end
  end

  def install
    bin.install "maenifold"
    (share/"maenifold").install "assets" if File.directory?("assets")
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
