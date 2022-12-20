// SPDX-License-Identifier: BSD-3-Clause
// See Notices.txt for copyright information

extern crate cc;
extern crate cc_version;

use cc_version::{cc_version, Version};
use std::env;
use std::path::{Path, PathBuf};

#[allow(unused, non_camel_case_types)]
#[derive(Clone, Copy)]
enum Specialization {
    X8086,
    X8086_SSE,
    ARM_VFPv2,
    ARM_VFPv2_DefaultNaN,
    RISCV,
}

impl Specialization {
    fn to_str(&self) -> &'static str {
        match self {
            Specialization::X8086 => "8086",
            Specialization::X8086_SSE => "8086-SSE",
            Specialization::ARM_VFPv2 => "ARM-VFPv2",
            Specialization::ARM_VFPv2_DefaultNaN => "ARM-VFPv2-defaultNaN",
            Specialization::RISCV => "RISCV",
        }
    }
}

#[allow(unused, non_camel_case_types)]
#[derive(Clone, Copy)]
enum BuildTarget {
    Linux_386_GCC,
    Linux_386_SSE2_GCC,
    Linux_Arm_VFPv2_GCC,
    Linux_x86_64_GCC,
    Wasm_Clang,
    Win32_MinGW,
    Win32_SSE2_MinGW,
    Win64_MinGW_w64,
}

impl BuildTarget {
    fn to_str(&self) -> &'static str {
        match self {
            BuildTarget::Linux_386_GCC => "Linux-386-GCC",
            BuildTarget::Linux_386_SSE2_GCC => "Linux-386-SSE2-GCC",
            BuildTarget::Linux_Arm_VFPv2_GCC => "Linux-ARM-VFPv2-GCC",
            BuildTarget::Linux_x86_64_GCC => "Linux-x86_64-GCC",
            BuildTarget::Wasm_Clang => "Wasm-Clang",
            BuildTarget::Win32_MinGW => "Win32-MinGW",
            BuildTarget::Win32_SSE2_MinGW => "Win32-SSE2-MinGW",
            BuildTarget::Win64_MinGW_w64 => "Win64-MinGW-w64",
        }
    }
}

struct Defines {
    softfloat_round_odd: bool,
    inline_level: Option<i32>,
    softfloat_fast_div_32_to_16: bool,
    softfloat_fast_div_64_to_32: bool,
    softfloat_fast_int64: bool,
}

struct PlatformCfg<'a> {
    softfloat_source: &'a Path,
    softfloat_build: &'a Path,
    primitive_sources: &'a [&'a str],
    specialize_sources: &'a [&'a str],
    other_sources: &'a [&'a str],
    thread_local: Option<&'a str>,
}

impl<'a> PlatformCfg<'a> {
    fn configure_platform(
        &self,
        builder: &mut cc::Build,
        spec: Specialization,
        target: BuildTarget,
        defines: Defines,
    ) {
        let specialized_source_path = self.softfloat_source.join(Path::new(spec.to_str()));
        builder
            .include(self.softfloat_build.join(Path::new(target.to_str())))
            .include(&specialized_source_path);

        if defines.softfloat_round_odd {
            builder.define("SOFTFLOAT_ROUND_ODD", None);
        }

        if let Some(level) = defines.inline_level {
            builder.define("INLINE_LEVEL", Some(level.to_string().as_str()));
        }

        if defines.softfloat_fast_div_32_to_16 {
            builder.define("SOFTFLOAT_FAST_DIV32TO16", None);
        }

        if defines.softfloat_fast_div_64_to_32 {
            builder.define("SOFTFLOAT_FAST_DIV64TO32", None);
        }

        if defines.softfloat_fast_int64 {
            builder.define("SOFTFLOAT_FAST_INT64", None);
        }

        builder
            .define("THREAD_LOCAL", self.thread_local)
            .files(
                self.primitive_sources
                    .iter()
                    .chain(self.other_sources.iter())
                    .map(|file| self.softfloat_source.join(Path::new(file))),
            )
            .files(
                self.specialize_sources
                    .iter()
                    .map(|file| specialized_source_path.join(Path::new(file))),
            );
    }
}

fn main() {
    //
    // Compile aliases for C stdint types
    // `c99` crate doesn't support cross-compilation, so we do all the stuff ourselves
    //
    println!("cargo:rerun-if-changed=c99-types.h");

    let bindings = bindgen::Builder::default()
        .header("c99-types.h")
        .use_core()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate C99 type aliases");

    bindings
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("c99-types.rs"))
        .expect("Couldn't write bindings");

    //
    // Compile berkeley-softfloat manually,
    // makefiles provided support only in-tree compilation, which isn't what we want
    //

    let mut builder = cc::Build::new();
    // FIXME: use more stable way to detect `thread_local` alias
    let tool = builder.get_compiler();
    let thread_local = if tool.is_like_gnu() {
        let version = cc_version(&tool).expect("Failed to detect GCC version");

        // GCC 4.9 supports _Thread_local
        if version >= Version::parse("4.9").unwrap() {
            Some("_Thread_local")
        } else {
            Some("__thread")
        }
    } else {
        Some("_Thread_local")
    };

    let softfloat_base = Path::new("berkeley-softfloat-3");
    let softfloat_source = softfloat_base.join(Path::new("source"));
    let softfloat_build = softfloat_base.join(Path::new("build"));

    let primitive_sources = [
        "s_eq128.c",
        "s_le128.c",
        "s_lt128.c",
        "s_shortShiftLeft128.c",
        "s_shortShiftRight128.c",
        "s_shortShiftRightJam64.c",
        "s_shortShiftRightJam64Extra.c",
        "s_shortShiftRightJam128.c",
        "s_shortShiftRightJam128Extra.c",
        "s_shiftRightJam32.c",
        "s_shiftRightJam64.c",
        "s_shiftRightJam64Extra.c",
        "s_shiftRightJam128.c",
        "s_shiftRightJam128Extra.c",
        "s_shiftRightJam256M.c",
        "s_countLeadingZeros8.c",
        "s_countLeadingZeros16.c",
        "s_countLeadingZeros32.c",
        "s_countLeadingZeros64.c",
        "s_add128.c",
        "s_add256M.c",
        "s_sub128.c",
        "s_sub256M.c",
        "s_mul64ByShifted32To128.c",
        "s_mul64To128.c",
        "s_mul128By32.c",
        "s_mul128To256M.c",
        "s_approxRecip_1Ks.c",
        "s_approxRecip32_1.c",
        "s_approxRecipSqrt_1Ks.c",
        "s_approxRecipSqrt32_1.c",
    ];
    let specialize_sources = [
        "softfloat_raiseFlags.c",
        "s_f16UIToCommonNaN.c",
        "s_commonNaNToF16UI.c",
        "s_propagateNaNF16UI.c",
        "s_f32UIToCommonNaN.c",
        "s_commonNaNToF32UI.c",
        "s_propagateNaNF32UI.c",
        "s_f64UIToCommonNaN.c",
        "s_commonNaNToF64UI.c",
        "s_propagateNaNF64UI.c",
        "extF80M_isSignalingNaN.c",
        "s_extF80UIToCommonNaN.c",
        "s_commonNaNToExtF80UI.c",
        "s_propagateNaNExtF80UI.c",
        "f128M_isSignalingNaN.c",
        "s_f128UIToCommonNaN.c",
        "s_commonNaNToF128UI.c",
        "s_propagateNaNF128UI.c",
    ];
    let other_sources = [
        "s_roundToUI32.c",
        "s_roundToUI64.c",
        "s_roundToI32.c",
        "s_roundToI64.c",
        "s_normSubnormalF16Sig.c",
        "s_roundPackToF16.c",
        "s_normRoundPackToF16.c",
        "s_addMagsF16.c",
        "s_subMagsF16.c",
        "s_mulAddF16.c",
        "s_normSubnormalF32Sig.c",
        "s_roundPackToF32.c",
        "s_normRoundPackToF32.c",
        "s_addMagsF32.c",
        "s_subMagsF32.c",
        "s_mulAddF32.c",
        "s_normSubnormalF64Sig.c",
        "s_roundPackToF64.c",
        "s_normRoundPackToF64.c",
        "s_addMagsF64.c",
        "s_subMagsF64.c",
        "s_mulAddF64.c",
        "s_normSubnormalExtF80Sig.c",
        "s_roundPackToExtF80.c",
        "s_normRoundPackToExtF80.c",
        "s_addMagsExtF80.c",
        "s_subMagsExtF80.c",
        "s_normSubnormalF128Sig.c",
        "s_roundPackToF128.c",
        "s_normRoundPackToF128.c",
        "s_addMagsF128.c",
        "s_subMagsF128.c",
        "s_mulAddF128.c",
        "softfloat_state.c",
        "ui32_to_f16.c",
        "ui32_to_f32.c",
        "ui32_to_f64.c",
        "ui32_to_extF80.c",
        "ui32_to_extF80M.c",
        "ui32_to_f128.c",
        "ui32_to_f128M.c",
        "ui64_to_f16.c",
        "ui64_to_f32.c",
        "ui64_to_f64.c",
        "ui64_to_extF80.c",
        "ui64_to_extF80M.c",
        "ui64_to_f128.c",
        "ui64_to_f128M.c",
        "i32_to_f16.c",
        "i32_to_f32.c",
        "i32_to_f64.c",
        "i32_to_extF80.c",
        "i32_to_extF80M.c",
        "i32_to_f128.c",
        "i32_to_f128M.c",
        "i64_to_f16.c",
        "i64_to_f32.c",
        "i64_to_f64.c",
        "i64_to_extF80.c",
        "i64_to_extF80M.c",
        "i64_to_f128.c",
        "i64_to_f128M.c",
        "f16_to_ui32.c",
        "f16_to_ui64.c",
        "f16_to_i32.c",
        "f16_to_i64.c",
        "f16_to_ui32_r_minMag.c",
        "f16_to_ui64_r_minMag.c",
        "f16_to_i32_r_minMag.c",
        "f16_to_i64_r_minMag.c",
        "f16_to_f32.c",
        "f16_to_f64.c",
        "f16_to_extF80.c",
        "f16_to_extF80M.c",
        "f16_to_f128.c",
        "f16_to_f128M.c",
        "f16_roundToInt.c",
        "f16_add.c",
        "f16_sub.c",
        "f16_mul.c",
        "f16_mulAdd.c",
        "f16_div.c",
        "f16_rem.c",
        "f16_sqrt.c",
        "f16_eq.c",
        "f16_le.c",
        "f16_lt.c",
        "f16_eq_signaling.c",
        "f16_le_quiet.c",
        "f16_lt_quiet.c",
        "f16_isSignalingNaN.c",
        "f32_to_ui32.c",
        "f32_to_ui64.c",
        "f32_to_i32.c",
        "f32_to_i64.c",
        "f32_to_ui32_r_minMag.c",
        "f32_to_ui64_r_minMag.c",
        "f32_to_i32_r_minMag.c",
        "f32_to_i64_r_minMag.c",
        "f32_to_f16.c",
        "f32_to_f64.c",
        "f32_to_extF80.c",
        "f32_to_extF80M.c",
        "f32_to_f128.c",
        "f32_to_f128M.c",
        "f32_roundToInt.c",
        "f32_add.c",
        "f32_sub.c",
        "f32_mul.c",
        "f32_mulAdd.c",
        "f32_div.c",
        "f32_rem.c",
        "f32_sqrt.c",
        "f32_eq.c",
        "f32_le.c",
        "f32_lt.c",
        "f32_eq_signaling.c",
        "f32_le_quiet.c",
        "f32_lt_quiet.c",
        "f32_isSignalingNaN.c",
        "f64_to_ui32.c",
        "f64_to_ui64.c",
        "f64_to_i32.c",
        "f64_to_i64.c",
        "f64_to_ui32_r_minMag.c",
        "f64_to_ui64_r_minMag.c",
        "f64_to_i32_r_minMag.c",
        "f64_to_i64_r_minMag.c",
        "f64_to_f16.c",
        "f64_to_f32.c",
        "f64_to_extF80.c",
        "f64_to_extF80M.c",
        "f64_to_f128.c",
        "f64_to_f128M.c",
        "f64_roundToInt.c",
        "f64_add.c",
        "f64_sub.c",
        "f64_mul.c",
        "f64_mulAdd.c",
        "f64_div.c",
        "f64_rem.c",
        "f64_sqrt.c",
        "f64_eq.c",
        "f64_le.c",
        "f64_lt.c",
        "f64_eq_signaling.c",
        "f64_le_quiet.c",
        "f64_lt_quiet.c",
        "f64_isSignalingNaN.c",
        "extF80_to_ui32.c",
        "extF80_to_ui64.c",
        "extF80_to_i32.c",
        "extF80_to_i64.c",
        "extF80_to_ui32_r_minMag.c",
        "extF80_to_ui64_r_minMag.c",
        "extF80_to_i32_r_minMag.c",
        "extF80_to_i64_r_minMag.c",
        "extF80_to_f16.c",
        "extF80_to_f32.c",
        "extF80_to_f64.c",
        "extF80_to_f128.c",
        "extF80_roundToInt.c",
        "extF80_add.c",
        "extF80_sub.c",
        "extF80_mul.c",
        "extF80_div.c",
        "extF80_rem.c",
        "extF80_sqrt.c",
        "extF80_eq.c",
        "extF80_le.c",
        "extF80_lt.c",
        "extF80_eq_signaling.c",
        "extF80_le_quiet.c",
        "extF80_lt_quiet.c",
        "extF80_isSignalingNaN.c",
        "extF80M_to_ui32.c",
        "extF80M_to_ui64.c",
        "extF80M_to_i32.c",
        "extF80M_to_i64.c",
        "extF80M_to_ui32_r_minMag.c",
        "extF80M_to_ui64_r_minMag.c",
        "extF80M_to_i32_r_minMag.c",
        "extF80M_to_i64_r_minMag.c",
        "extF80M_to_f16.c",
        "extF80M_to_f32.c",
        "extF80M_to_f64.c",
        "extF80M_to_f128M.c",
        "extF80M_roundToInt.c",
        "extF80M_add.c",
        "extF80M_sub.c",
        "extF80M_mul.c",
        "extF80M_div.c",
        "extF80M_rem.c",
        "extF80M_sqrt.c",
        "extF80M_eq.c",
        "extF80M_le.c",
        "extF80M_lt.c",
        "extF80M_eq_signaling.c",
        "extF80M_le_quiet.c",
        "extF80M_lt_quiet.c",
        "f128_to_ui32.c",
        "f128_to_ui64.c",
        "f128_to_i32.c",
        "f128_to_i64.c",
        "f128_to_ui32_r_minMag.c",
        "f128_to_ui64_r_minMag.c",
        "f128_to_i32_r_minMag.c",
        "f128_to_i64_r_minMag.c",
        "f128_to_f16.c",
        "f128_to_f32.c",
        "f128_to_extF80.c",
        "f128_to_f64.c",
        "f128_roundToInt.c",
        "f128_add.c",
        "f128_sub.c",
        "f128_mul.c",
        "f128_mulAdd.c",
        "f128_div.c",
        "f128_rem.c",
        "f128_sqrt.c",
        "f128_eq.c",
        "f128_le.c",
        "f128_lt.c",
        "f128_eq_signaling.c",
        "f128_le_quiet.c",
        "f128_lt_quiet.c",
        "f128_isSignalingNaN.c",
        "f128M_to_ui32.c",
        "f128M_to_ui64.c",
        "f128M_to_i32.c",
        "f128M_to_i64.c",
        "f128M_to_ui32_r_minMag.c",
        "f128M_to_ui64_r_minMag.c",
        "f128M_to_i32_r_minMag.c",
        "f128M_to_i64_r_minMag.c",
        "f128M_to_f16.c",
        "f128M_to_f32.c",
        "f128M_to_extF80M.c",
        "f128M_to_f64.c",
        "f128M_roundToInt.c",
        "f128M_add.c",
        "f128M_sub.c",
        "f128M_mul.c",
        "f128M_mulAdd.c",
        "f128M_div.c",
        "f128M_rem.c",
        "f128M_sqrt.c",
        "f128M_eq.c",
        "f128M_le.c",
        "f128M_lt.c",
        "f128M_eq_signaling.c",
        "f128M_le_quiet.c",
        "f128M_lt_quiet.c",
    ];

    let platform_cfg = PlatformCfg {
        softfloat_source: &softfloat_source,
        softfloat_build: &softfloat_build,
        primitive_sources: &primitive_sources,
        specialize_sources: &specialize_sources,
        other_sources: &other_sources,
        thread_local,
    };

    if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        platform_cfg.configure_platform(
            &mut builder,
            Specialization::X8086_SSE,
            BuildTarget::Linux_x86_64_GCC,
            Defines {
                softfloat_round_odd: true,
                inline_level: Some(5),
                softfloat_fast_div_32_to_16: true,
                softfloat_fast_div_64_to_32: true,
                softfloat_fast_int64: true,
            },
        );
    } else if cfg!(all(target_arch = "wasm32")) {
        platform_cfg.configure_platform(
            &mut builder,
            Specialization::X8086,
            BuildTarget::Wasm_Clang,
            Defines {
                softfloat_round_odd: true,
                inline_level: Some(5),
                softfloat_fast_div_32_to_16: true,
                softfloat_fast_div_64_to_32: true,
                softfloat_fast_int64: true,
            },
        );
    } else {
        unimplemented!("build rules are not implemented for the current target_arch and target_os");
    }
    if env::var("OPT_LEVEL").unwrap() == "0" {
        builder.opt_level(1); // work around softfloat bug with no definition for inline functions
    }
    builder
        .include(softfloat_source.join(Path::new("include")))
        .file(Path::new("helper.c"))
        .warnings(false)
        .compile("softfloat-sys");
}
