set -e
mkdir -p downloads
curl -fSLs --retry 5 --retry-delay 2 -o downloads/dmg-acid2.gb "https://github.com/mattcurrie/dmg-acid2/releases/download/v1.0/dmg-acid2.gb"&
(curl -fSLs --retry 5 --retry-delay 2 -o downloads/mts-20240926-1737-443f6e1.zip "https://gekkio.fi/files/mooneye-test-suite/mts-20240926-1737-443f6e1/mts-20240926-1737-443f6e1.zip" && unzip -q downloads/mts-20240926-1737-443f6e1.zip -d downloads)&
(curl -fSLs --retry 5 --retry-delay 2 -o downloads/blargg.zip "https://github.com/retrio/gb-test-roms/archive/refs/heads/master.zip" && unzip -q downloads/blargg.zip -d downloads)&
(curl -fSLs --retry 5 --retry-delay 2 -o downloads/mealybug-tearoom-tests.zip https://github.com/mattcurrie/mealybug-tearoom-tests/raw/70e88fb90b59d19dfbb9c3ac36c64105202bb1f4/mealybug-tearoom-tests.zip && mkdir downloads/mealybug-tearoom-tests && unzip -q downloads/mealybug-tearoom-tests.zip -d downloads/mealybug-tearoom-tests)&
wait
