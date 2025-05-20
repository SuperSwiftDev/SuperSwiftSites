set -e

# ./scripts/ssio.sh compile --root sample --template sample/base.html --input "sample/pages/**/*.html" --output output --pretty-print=false
./scripts/ssio.sh compile --root sample --template sample/base.html --input "sample/pages/**/*.html" --output output

# cargo run -- compile \
#     --root sample \
#     --template sample/base.html \
#     --input "sample/pages/**/*.html" \
#     --output sample/output
