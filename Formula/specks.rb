# Homebrew formula for specks
#
# To install:
#   brew tap kocienda/specks https://github.com/kocienda/specks
#   brew install specks
#
# This formula downloads prebuilt binaries from GitHub Releases.
# The version and checksums are automatically updated by CI on each release.

class Specks < Formula
  desc "From ideas to implementation via multi-agent orchestration"
  homepage "https://github.com/kocienda/specks"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kocienda/specks/releases/download/v#{version}/specks-#{version}-macos-arm64.tar.gz"
      # SHA256 ARM64: PLACEHOLDER_ARM64_SHA256
      sha256 "PLACEHOLDER_ARM64_SHA256"
    else
      url "https://github.com/kocienda/specks/releases/download/v#{version}/specks-#{version}-macos-x86_64.tar.gz"
      # SHA256 X86_64: PLACEHOLDER_X86_64_SHA256
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  def install
    bin.install "bin/specks"

    # Install skills to share directory
    # Skills end up at #{HOMEBREW_PREFIX}/share/specks/skills/
    (share/"specks").install "share/specks/skills"
  end

  def caveats
    <<~EOS
      Claude Code skills have been installed to:
        #{HOMEBREW_PREFIX}/share/specks/skills/

      To use /specks-plan and /specks-execute in your projects, run:
        specks setup claude

      This will copy the skills to your project's .claude/skills/ directory.
      You can also run this during `specks init` for new projects.
    EOS
  end

  test do
    system "#{bin}/specks", "--version"
    system "#{bin}/specks", "setup", "claude", "--check"
  end
end
