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
  version "0.2.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/specks-dev/specks/releases/download/v#{version}/specks-#{version}-macos-arm64.tar.gz"
      # SHA256 ARM64: f4669f5b579479cdeb4ab3bed7730102e5b0ccfeafb4fab9e4014783b6f29723
      sha256 "f4669f5b579479cdeb4ab3bed7730102e5b0ccfeafb4fab9e4014783b6f29723"
    else
      url "https://github.com/specks-dev/specks/releases/download/v#{version}/specks-#{version}-macos-x86_64.tar.gz"
      # SHA256 X86_64: 296a10f3c64b1375e878be9fa21a432c2a015235012ebf545047ac7445a122d0
      sha256 "296a10f3c64b1375e878be9fa21a432c2a015235012ebf545047ac7445a122d0"
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
