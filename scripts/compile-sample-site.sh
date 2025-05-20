set -e

# ./scripts/ssio.sh compile --root sample --template sample/base.html --input "sample/pages/**/*.html" --output output

./scripts/ssio.sh build --manifest sample/site.toml
