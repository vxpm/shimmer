jakubs_tests := "https://github.com/JaCzekanski/ps1-tests/releases/download/build-158/tests.zip"

amidogs_cpu := "https://psx.amidog.se/lib/exe/fetch.php?media=psx:download:psxtest_cpu.zip"
amidogs_cpx := "https://psx.amidog.se/lib/exe/fetch.php?media=psx:download:psxtest_cpx.zip"

# Lists all recipes
list:
    @just --list

# Downloads tests and puts them in the resources/tests directory
get-tests:
    @echo -e "=> Downloading Jakub's ps1-tests..."
    @wget {{jakubs_tests}} --output-document resources/tests/jakubs.zip -q --show-progress

    @echo -e "=> Extracting Jakub's ps1-tests..."
    @unzip -n -q resources/tests/jakubs.zip -d resources/tests/jakubs
    @rm resources/tests/jakubs.zip

    @echo -e "\n=> Downloading Amidog's CPU tests..."
    @wget {{amidogs_cpu}} --output-document resources/tests/amidogs_cpu.zip -q --show-progress
    @wget {{amidogs_cpx}} --output-document resources/tests/amidogs_cpx.zip -q --show-progress

    @echo -e "Extracting Amidog's CPU tests..."
    @unzip -n -q resources/tests/amidogs_cpu.zip -d resources/tests/amidogs
    @unzip -n -q resources/tests/amidogs_cpx.zip -d resources/tests/amidogs
    @rm resources/tests/amidogs_cpu.zip
    @rm resources/tests/amidogs_cpx.zip

    @echo -e "\n=> Downloading Amidog's CPU tests..."
    @wget {{amidogs_cpu}} --output-document resources/tests/amidogs_cpu.zip -q --show-progress
    @wget {{amidogs_cpx}} --output-document resources/tests/amidogs_cpx.zip -q --show-progress

    @echo -e "Extracting Amidog's CPU tests..."
    @unzip -n -q resources/tests/amidogs_cpu.zip -d resources/tests/amidogs
    @unzip -n -q resources/tests/amidogs_cpx.zip -d resources/tests/amidogs
    @rm resources/tests/amidogs_cpu.zip
    @rm resources/tests/amidogs_cpx.zip

    @echo -e "\n{{BOLD}}=> All done!{{NORMAL}}"
