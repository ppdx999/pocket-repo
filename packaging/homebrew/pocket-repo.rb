class PocketRepo < Formula
  desc "Mobile-first web app for browsing Git repositories from your phone"
  homepage "https://github.com/ppdx999/pocket-repo"
  license "MIT"

  # Install the latest commit with `brew install --HEAD`.
  head "https://github.com/ppdx999/pocket-repo.git", branch: "main"

  # For a tagged release, fill these in (and users can install without --HEAD):
  #   url "https://github.com/ppdx999/pocket-repo/archive/refs/tags/v0.1.0.tar.gz"
  #   sha256 "<shasum -a 256 of the tarball>"
  #   version "0.1.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  # `brew services start pocket-repo` runs this as a launchd user agent. With no
  # arguments the binary reads ~/.config/pocket-repo/config.toml.
  service do
    run [opt_bin/"pocket-repo"]
    keep_alive true
    working_dir Dir.home
    log_path var/"log/pocket-repo.log"
    error_log_path var/"log/pocket-repo.log"
  end

  test do
    assert_match "pocket-repo", shell_output("#{bin}/pocket-repo --version")
  end
end
