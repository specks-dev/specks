# Homebrew formula for specks
#
# To install:
#   brew tap specks-dev/specks https://github.com/specks-dev/specks
#   brew install specks
#
# This formula downloads prebuilt binaries from GitHub Releases.
# The version and checksums are automatically updated by CI on each release.

class Specks < Formula
  desc "From ideas to implementation via multi-agent orchestration"
  homepage "https://github.com/specks-dev/specks"
  version "0.2.5"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/specks-dev/specks/releases/download/v#{version}/specks-#{version}-macos-arm64.tar.gz"
      # SHA256 ARM64: e87a863a56f1107934e2a551116906a4614ec88820986a82a005117320040d64
      sha256 "e87a863a56f1107934e2a551116906a4614ec88820986a82a005117320040d64"
    else
      url "https://github.com/specks-dev/specks/releases/download/v#{version}/specks-#{version}-macos-x86_64.tar.gz"
      # SHA256 X86_64: 280cb5102ce0dcb5195e6797ec2525e8e807ba5bbfb6cd1193af5e2c43ca54c6
      sha256 "280cb5102ce0dcb5195e6797ec2525e8e807ba5bbfb6cd1193af5e2c43ca54c6"
    end
  end

  def install
    bin.install "bin/specks"

    # Install skills to share directory
    # Skills end up at #{HOMEBREW_PREFIX}/share/specks/skills/
    (share/"specks").install "share/specks/skills"

    # Install agents to share directory
    # Agents end up at #{HOMEBREW_PREFIX}/share/specks/agents/
    (share/"specks").install "share/specks/agents"
  end

  def caveats
    <<~EOS
      Specks agents have been installed to:
        #{HOMEBREW_PREFIX}/share/specks/agents/

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
