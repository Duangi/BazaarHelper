#!/bin/bash
set -e

# Bundle ONNX Runtime and dependencies into macOS .app
# This script copies all necessary dylibs into the app bundle and fixes their paths

APP_DIR="target/release/bundle/macos/BazaarHelper.app"
FRAMEWORKS_DIR="$APP_DIR/Contents/Frameworks"

echo "📦 Bundling ONNX Runtime libraries into $APP_DIR"

# Check if app exists
if [ ! -d "$APP_DIR" ]; then
    echo "❌ Error: $APP_DIR not found"
    exit 1
fi

# Create Frameworks directory if it doesn't exist
mkdir -p "$FRAMEWORKS_DIR"

# Function to copy a dylib and fix its paths
copy_and_fix_dylib() {
    local lib_path=$1
    local lib_name=$(basename "$lib_path")
    
    if [ ! -f "$lib_path" ]; then
        echo "⚠️  Warning: $lib_path not found, skipping"
        return
    fi
    
    echo "  Copying $lib_name..."
    cp "$lib_path" "$FRAMEWORKS_DIR/"
    
    # Fix the install name to use @executable_path
    install_name_tool -id "@executable_path/../Frameworks/$lib_name" "$FRAMEWORKS_DIR/$lib_name" 2>/dev/null || true
}

# Copy ONNX Runtime libraries from Homebrew
if [ -d "/opt/homebrew/lib" ]; then
    HOMEBREW_LIB="/opt/homebrew/lib"
    
    # Main ONNX Runtime library
    copy_and_fix_dylib "$HOMEBREW_LIB/libonnxruntime.dylib"
    copy_and_fix_dylib "$HOMEBREW_LIB/libonnxruntime.1.19.2.dylib"
    
    # ONNX dependencies
    copy_and_fix_dylib "$HOMEBREW_LIB/libonnx.dylib"
    copy_and_fix_dylib "$HOMEBREW_LIB/libonnx_proto.dylib"
    
    # Protobuf
    copy_and_fix_dylib "$HOMEBREW_LIB/libprotobuf-lite.dylib"
    copy_and_fix_dylib "$HOMEBREW_LIB/libprotobuf.dylib"
    
    # RE2
    copy_and_fix_dylib "$HOMEBREW_LIB/libre2.dylib"
    
    echo "✅ Libraries copied to Frameworks directory"
else
    echo "⚠️  Warning: Homebrew lib directory not found at /opt/homebrew/lib"
fi

# Fix paths in the main executable
EXECUTABLE="$APP_DIR/Contents/MacOS/BazaarHelper"
if [ -f "$EXECUTABLE" ]; then
    echo "🔧 Fixing library paths in executable..."
    
    # Update references to use @executable_path
    install_name_tool -change "/opt/homebrew/lib/libonnxruntime.dylib" "@executable_path/../Frameworks/libonnxruntime.dylib" "$EXECUTABLE" 2>/dev/null || true
    install_name_tool -change "/opt/homebrew/lib/libonnxruntime.1.19.2.dylib" "@executable_path/../Frameworks/libonnxruntime.1.19.2.dylib" "$EXECUTABLE" 2>/dev/null || true
    
    echo "✅ Executable paths updated"
else
    echo "⚠️  Warning: Executable not found at $EXECUTABLE"
fi

# Fix inter-library dependencies
echo "🔧 Fixing inter-library dependencies..."
for dylib in "$FRAMEWORKS_DIR"/*.dylib; do
    if [ -f "$dylib" ]; then
        install_name_tool -change "/opt/homebrew/lib/libonnx.dylib" "@executable_path/../Frameworks/libonnx.dylib" "$dylib" 2>/dev/null || true
        install_name_tool -change "/opt/homebrew/lib/libonnx_proto.dylib" "@executable_path/../Frameworks/libonnx_proto.dylib" "$dylib" 2>/dev/null || true
        install_name_tool -change "/opt/homebrew/lib/libprotobuf-lite.dylib" "@executable_path/../Frameworks/libprotobuf-lite.dylib" "$dylib" 2>/dev/null || true
        install_name_tool -change "/opt/homebrew/lib/libprotobuf.dylib" "@executable_path/../Frameworks/libprotobuf.dylib" "$dylib" 2>/dev/null || true
        install_name_tool -change "/opt/homebrew/lib/libre2.dylib" "@executable_path/../Frameworks/libre2.dylib" "$dylib" 2>/dev/null || true
    fi
done

echo "✅ ONNX Runtime bundling complete!"
echo "📊 Bundled libraries:"
ls -lh "$FRAMEWORKS_DIR"
