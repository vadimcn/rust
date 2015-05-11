# x86_64-pc-windows-msvc configuration
CC_x86_64-pc-windows-msvc=$(CFG_CL) -nologo
LINK_x86_64-pc-windows-msvc=$(CFG_LINK) -nologo
CXX_x86_64-pc-windows-msvc=$(CFG_CL) -nologo
CPP_x86_64-pc-windows-msvc=$(CFG_CL) -nologo
AR_x86_64-pc-windows-msvc=$(CFG_LIB) -nologo
CFG_LIB_NAME_x86_64-pc-windows-msvc=$(1).dll
CFG_STATIC_LIB_NAME_x86_64-pc-windows-msvc=$(1).lib
CFG_LIB_GLOB_x86_64-pc-windows-msvc=$(1)-*.dll
CFG_LIB_DSYM_GLOB_x86_64-pc-windows-msvc=$(1)-*.dylib.dSYM
CFG_JEMALLOC_CFLAGS_x86_64-pc-windows-msvc :=
CFG_GCCISH_CFLAGS_x86_64-pc-windows-msvc :=
CFG_GCCISH_CXXFLAGS_x86_64-pc-windows-msvc :=
CFG_GCCISH_LINK_FLAGS_x86_64-pc-windows-msvc :=
CFG_GCCISH_DEF_FLAG_x86_64-pc-windows-msvc :=
CFG_LLC_FLAGS_x86_64-pc-windows-msvc :=
CFG_INSTALL_NAME_x86_64-pc-windows-msvc =
CFG_EXE_SUFFIX_x86_64-pc-windows-msvc := .exe
CFG_WINDOWSY_x86_64-pc-windows-msvc := 1
CFG_UNIXY_x86_64-pc-windows-msvc :=
CFG_LDPATH_x86_64-pc-windows-msvc :=
CFG_RUN_x86_64-pc-windows-msvc=$(2)
CFG_RUN_TARG_x86_64-pc-windows-msvc=$(call CFG_RUN_x86_64-pc-windows-msvc,,$(2))
CFG_GNU_TRIPLE_x86_64-pc-windows-msvc := x86_64-pc-win32

export INCLUDE := $(CFG_MSVC_INCLUDE)
export LIB := $(CFG_MSVC_LIB)
NATIVE_TOOL_DEPS_core_T_x86_64-pc-windows-msvc += llvm-ar.exe
