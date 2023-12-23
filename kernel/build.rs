use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

const COMPILER: &'static str = "x86_64-elf-gcc";
const ARCHIVER: &'static str = "x86_64-elf-ar";

const C_FLAGS: &'static [&'static str] = &[
    "-c", "-ffreestanding",
    "-m64", "-mcmodel=kernel", "-mno-red-zone", "-mabi=sysv", "-msoft-float",
    "-mno-sse", "-mno-mmx", "-mno-sse2", "-mno-3dnow", "-mno-avx"
];

const C_SRC: &'static [&'static str] = &[
    "c_src/entry.c",
    "c_src/hang.c",
    "c_src/int.c",
    "c_src/io.c",
    "c_src/mm.c",
    "c_src/types.c",
];

const ASM_FLAGS: &'static [&'static str] = &[
    "-felf64",
    "-w+orphan-labels"
];

const ASM_SRC: &'static [&'static str] = &[
    "asm_src/boot.asm",
    "asm_src/int.asm",
    "asm_src/mm.asm",
    "asm_src/task.asm",
];

fn build_c() {
    let mut cc_build = cc::Build::new();
    cc_build
        .compiler(COMPILER)
        .archiver(ARCHIVER)
        .no_default_flags(true)
        .include("include");

    for c_flag in C_FLAGS {
        cc_build.flag(c_flag);
    }

    cc_build.files(C_SRC);

    cc_build.compile("sparkle_c");
}

fn run_cmd(cmd: &mut Command) {
    println!("running {:?}", cmd);
    println!("{}", cmd.status().unwrap())
}

fn build_asm() {
    // scuffed
    // TODO: archive the objects into a library and link that instead
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let ar_path = out_dir.join("libsparkle_asm.a");
    let _ =fs::remove_file(&ar_path);
    let mut ar_cmd = Command::new(ARCHIVER);
    ar_cmd.env("ZERO_AR_DATE", "1");
    ar_cmd.arg("cq").arg(&ar_path);

    for file in ASM_SRC {
        let mut dst = out_dir.join(file);
        dst.set_extension("o");
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        let mut cmd = Command::new("nasm");
        cmd.args(ASM_FLAGS).arg("-o").arg(&dst).arg(file);
        run_cmd(&mut cmd);

        ar_cmd.arg(&dst);
    }

    run_cmd(&mut ar_cmd);
    run_cmd(Command::new(ARCHIVER).arg("s").arg(&ar_path));
    println!("cargo:rustc-link-lib=static=sparkle_asm");
}

fn mark_used_files() {
    for file in C_SRC.iter().chain(ASM_SRC.iter()) {
        println!("cargo:rerun-if-changed={}", file);
    }
}

fn main() {
    build_c();
    build_asm();
    mark_used_files();

    println!("cargo:rustc-link-arg=-Tlinker.ld")
}