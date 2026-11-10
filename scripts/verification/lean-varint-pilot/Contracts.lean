import VarintLeanPilot.Extraction

open Aeneas Aeneas.Std
open varint_lean_pilot

namespace VarintLeanPilot

/-- The minimal encoding of zero is one byte. -/
theorem encoded_len_zero : varint.encoded_len 0#u64 = RustM.ok 1#usize := by
  simp [varint.encoded_len]

end VarintLeanPilot
