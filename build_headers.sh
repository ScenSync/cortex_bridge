#!/bin/bash

# Fast build script for generating C header files from Rust FFI
# 针对开发环境的快速头文件生成脚本

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否在正确的目录
if [ ! -f "Cargo.toml" ]; then
    log_error "请在 easytier-bridge 目录运行此脚本"
    exit 1
fi

# 设置环境变量以加速编译
export CARGO_NET_GIT_FETCH_WITH_CLI=true
export RUSTC_WRAPPER=""

# 检查是否安装了 sccache
if command -v sccache &> /dev/null; then
    log_info "使用 sccache 加速编译"
    export RUSTC_WRAPPER=sccache
    # sccache 与增量编译冲突，禁用增量编译
    export CARGO_INCREMENTAL=0
    sccache --show-stats
else
    log_warning "建议安装 sccache 以加速编译: cargo install sccache"
    # 没有 sccache 时启用增量编译
    export CARGO_INCREMENTAL=1
fi

# Get number of CPU cores for parallel builds
CPU_CORES=$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "4")
# Ensure CPU_CORES is at least 1
if [ "$CPU_CORES" -le 0 ]; then
    CPU_CORES=1
fi
export CARGO_BUILD_JOBS=$CPU_CORES

# 解析命令行参数
VERBOSE=false
CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --no-clean)
            CLEAN=false
            shift
            ;;
        -h|--help)
            echo "用法: $0 [选项]"
            echo "选项:"
            echo "  -v, --verbose   详细输出"
            echo "  --no-clean      不清理旧文件"
            echo "  -h, --help      显示帮助信息"
            exit 0
            ;;
        *)
            log_error "未知参数: $1"
            exit 1
            ;;
    esac
done

# Clean old headers and target directories
if [ "$CLEAN" = true ]; then
    log_info "清理旧的头文件和目标目录..."
    rm -f easytier_bridge.h
    rm -f cortex_easytier_core.h cortex_easytier_web.h
    rm -rf include/
    rm -rf target/
fi

# Create include directory
mkdir -p include

log_info "开始生成 Cortex EasyTier Bridge 头文件..."
log_info "CPU 核心数: $CPU_CORES"
log_info "增量编译: $([ "$CARGO_INCREMENTAL" = "1" ] && echo "启用" || echo "禁用")"
log_info "详细输出: $([ "$VERBOSE" = true ] && echo "启用" || echo "禁用")"

# Function to build a subproject with optimizations
build_subproject() {
    local project_name=$1
    local header_name=$2
    local start_time=$(date +%s)
    
    log_info "构建 $project_name..."
    cd "$project_name"
    
    # 构建 cargo 命令
    local cargo_cmd="cargo build --jobs $CPU_CORES"
    if [ "$VERBOSE" = true ]; then
        cargo_cmd="$cargo_cmd --verbose"
    fi
    
    log_info "执行命令: $cargo_cmd"
    if eval "$cargo_cmd"; then
        local build_end_time=$(date +%s)
        local build_duration=$((build_end_time - start_time))
        log_success "$project_name 构建成功！耗时: ${build_duration}s"
    else
        log_error "$project_name 构建失败"
        cd ..
        return 1
    fi
    
    # Generate header file using cbindgen
    log_info "为 $project_name 生成头文件..."
    if cbindgen --config cbindgen.toml --crate "$project_name" --output "../include/$header_name"; then
        log_success "✓ $header_name 生成成功"
    else
        log_warning "警告: $header_name 生成失败"
    fi
    
    cd ..
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - start_time))
    log_info "$project_name 总耗时: ${total_duration}s"
}

# Build unified project with optimizations
log_info "开始优化构建..."
start_time=$(date +%s)

# Build the unified easytier-bridge
log_info "构建 easytier-bridge..."
local_start_time=$(date +%s)

# 构建 cargo 命令
cargo_cmd="cargo build --jobs $CPU_CORES"
if [ "$VERBOSE" = true ]; then
    cargo_cmd="$cargo_cmd --verbose"
fi

log_info "执行命令: $cargo_cmd"
if eval "$cargo_cmd"; then
    local_build_end_time=$(date +%s)
    local_build_duration=$((local_build_end_time - local_start_time))
    log_success "easytier-bridge 构建成功！耗时: ${local_build_duration}s"
else
    log_error "easytier-bridge 构建失败"
    exit 1
fi

# Generate header file using cbindgen
log_info "为 easytier-bridge 生成头文件..."
if cbindgen --config cbindgen.toml --crate easytier-bridge --output "include/easytier_bridge.h"; then
    log_success "✓ easytier_bridge.h 生成成功"
else
    log_error "easytier_bridge.h 生成失败"
    exit 1
fi

end_time=$(date +%s)
build_duration=$((end_time - start_time))

log_success "头文件生成完成！总耗时: ${build_duration}s"

# 显示 sccache 统计信息
if command -v sccache &> /dev/null && [ -n "$RUSTC_WRAPPER" ]; then
    echo
    log_info "sccache 统计信息:"
    sccache --show-stats
fi

log_info "生成的头文件:"
if ls -la include/ 2>/dev/null; then
    echo
    log_success "所有头文件已成功生成到 include/ 目录"
else
    log_warning "include/ 目录中未找到头文件"
fi
