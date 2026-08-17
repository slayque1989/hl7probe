class Hl7probe < Formula
  desc "Inspect and validate HL7 v2 messages"
  homepage "https://github.com/sudhi001/hl7probe"
  url "https://github.com/sudhi001/hl7probe/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "6850b0e5d07bed32b3613d4c7da50e0fc36542239a5ff5188b524494e9edda75"
  license "MIT"
  head "https://github.com/sudhi001/hl7probe.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    (testpath/"admit.hl7").write <<~HL7
      MSH|^~\\&|HIS|MERCY|LIS|LAB|20240115143200||ADT^A01^ADT_A01|MSG1|P|2.5.1\r
      EVN|A01|20240115143200||||20240115143000\r
      PID|1||123456^^^MERCY^MR||Smith^John||19850312|M|||1 Oak St^^Springfield^IL^62704\r
      PV1|1|I|ER^101^A|||1234^Adams^Alice||||||||||||V1\r
    HL7

    output = shell_output("#{bin}/hl7test #{testpath}/admit.hl7 --color never")
    assert_match "ADT^A01", output
    assert_match "Patient Name", output

    assert_equal "Smith", shell_output("#{bin}/hl7test -f PID-5.1 #{testpath}/admit.hl7").strip
    assert_match version.to_s, shell_output("#{bin}/hl7test --version")
  end
end
