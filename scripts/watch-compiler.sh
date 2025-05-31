set -e

# watchexec --ignore output/ -w . -e html -- scripts/compile.sh
watchexec --ignore-file .watchexec-ignore -w sample -e html -e css -- scripts/compile-sample-site.sh

