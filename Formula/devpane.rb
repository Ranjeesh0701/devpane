class Devpane < Formula
  desc "Terminal-first multi-process workspace and development dashboard"
  homepage "https://github.com/Ranjeesh0701/devpane"
  url "https://github.com/Ranjeesh0701/devpane/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "e1f2afa1f17bfa4b258e04f14934bd8bf04aa425feff30ca9d1bde10b29bde02"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "devpane", shell_output("#{bin}/devpane --help")
  end
end
