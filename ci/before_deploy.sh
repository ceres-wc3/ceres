#!/bin/bash
# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    # TODO Update this to build the artifacts that matter to you
    CERES_NOBUNDLE_CPP=1 cross rustc --manifest-path ceres-utils/Cargo.toml --bin ceres --release --target $TARGET -- -C lto

    if [ -e "target/$TARGET/release/ceres.exe" ]; then
        cp target/$TARGET/release/ceres.exe $stage/
    fi

    if [ -e "target/$TARGET/release/ceres" ]; then
        cp target/$TARGET/release/ceres $stage/
    fi

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
    cd $src

    rm -rf $stage
}

main
