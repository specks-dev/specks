# Homebrew formula for specks
#
# To install from a local tap:
#   brew tap username/specks path/to/homebrew
#   brew install specks
#
# Or install directly from the formula file:
#   brew install --build-from-source path/to/specks.rb
#
# This formula downloads prebuilt binaries from GitHub Releases.
# Update the version, url, and sha256 values when releasing new versions.

class Specks < Formula
  desc "From ideas to implementation via multi-agent orchestration"
  homepage "https://github.com/kocienda/specks"
  version "0.1.0"
  license "MIT"

  # Update these URLs and checksums for each release
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kocienda/specks/releases/download/v#{version}/specks-#{version}-macos-arm64.tar.gz"
      # sha256 "UPDATE_ARM64_SHA256_HERE"
    else
      url "https://github.com/kocienda/specks/releases/download/v#{version}/specks-#{version}-macos-x86_64.tar.gz"
      # sha256 "UPDATE_X86_64_SHA256_HERE"
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
