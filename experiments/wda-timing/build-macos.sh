#!/bin/bash
# Build-only experiment. Never signs, installs, launches, or submits phone input.
set -euo pipefail

if [[ "$(uname -s)" != Darwin ]]; then
  printf '%s\n' 'This build requires macOS and Xcode.' >&2
  exit 1
fi
if [[ $# -ne 1 || "$1" != /* || -e "$1" ]]; then
  printf '%s\n' 'Usage: bash build-macos.sh /absolute/fresh/output-directory' >&2
  exit 1
fi
experiment_dir="$(cd "$(dirname "$0")" && pwd)"
output="$1"
revision=3e8aa7de81f254dbb0876baa9e9173c16b55b3a0
mkdir -p "$output"
git clone --filter=blob:none --no-checkout https://github.com/appium/WebDriverAgent.git "$output/source"
git -C "$output/source" checkout --detach "$revision"
test "$(git -C "$output/source" rev-parse HEAD)" = "$revision"
git -C "$output/source" apply --check "$experiment_dir/runner-timing.patch"
git -C "$output/source" apply "$experiment_dir/runner-timing.patch"
xcodebuild -version > "$output/xcode-version.txt"
xcodebuild \
  -project "$output/source/WebDriverAgent.xcodeproj" \
  -scheme WebDriverAgentRunner \
  -configuration Release \
  -destination 'generic/platform=iOS' \
  -derivedDataPath "$output/build" \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO DEVELOPMENT_TEAM= \
  build-for-testing > "$output/xcodebuild.log" 2>&1
app="$output/build/Build/Products/Release-iphoneos/WebDriverAgentRunner-Runner.app"
test -d "$app"
mkdir "$output/Payload"
ditto "$app" "$output/Payload/WebDriverAgentRunner-Runner.app"
(
  cd "$output"
  ditto -c -k --keepParent Payload iMirror-WDA-timing-unsigned.ipa
  shasum -a 256 iMirror-WDA-timing-unsigned.ipa > SHA256SUMS.txt
)
cp "$output/source/LICENSE" "$output/WDA-LICENSE"
cp "$experiment_dir/runner-timing.patch" "$output/runner-timing.patch"
printf '%s\n' "$revision" > "$output/upstream-commit.txt"
printf '%s\n' 'UNSIGNED EXPERIMENT. Not installed or hardware validated.' > "$output/STATUS.txt"
printf '%s\n' "$output/iMirror-WDA-timing-unsigned.ipa"
