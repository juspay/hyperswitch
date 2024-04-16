#!/usr/bin/env bash
#
# Description: One click install for hyperswitch router
#
#
# Global config

if [[ "${TRACE-0}" == "1" ]]; then
    set -o xtrace
fi

RUST_MSRV=1.70.0
_DB_NAME="hyperswitch_db"
_DB_USER="db_user"
_DB_PASS="db_password"

OSTYPE=${OSTYPE:-}

PRE_INSTALL_MSG="Dependency install script.\n
The script assumes 'curl' and build essentials like gcc/clang are already installed.\n
\n
The script will\n
1. Install or update RUST using RUSTUP\n
2. Install Postgresql server and redis server, if found missing\n
3. Install diesel_cli program to setup database\n
4. Setup database and create necessary schema\n
"

POST_INSTALL_MSG="\n
Install was successful.\n
If rust was installed, restart the shell or configure current shell using 'SOURCE $HOME/.cargo/env'\n
"


# Used variables must be initialized
set -o nounset

# utilities
# convert semver to comparable integer
if [[ `id -u` -ne 0 ]]; then
    print_info "requires sudo"
    SUDO=sudo
else
    SUDO=""
fi

ver () {
    printf "%03d%03d%03d%03d" `echo "$1" | tr '.' ' '`;
}

PROGNAME=`basename $0`
print_info () {
    echo -e "$PROGNAME: $*"
}

err () {
    print_info "*ERROR*:" $*
    exit 1
}

need_cmd () {
    if ! command -v $1 > /dev/null
    then
        err "Command \"${1}\" not found. Bailing out"
    fi

}

prompt () {
    read -p "$*? [y/N] :" ANSWER
    case $ANSWER in
        [Yy]*) return 1;;
        *) return 0;;
    esac
}

init_start_postgres () {

    if [[ "${OSTYPE}" == "darwin"* ]]; then
        initdb -U postgres -D /var/lib/postgres/data
    elif command -v su > /dev/null; then
        $SUDO su -c "initdb -D /var/lib/postgres/data" postgres
    elif command -v sudo > /dev/null; then
        sudo -u postgres "initdb -D /var/lib/postgres/data"
    else
        err "Don't know how to switch to postgres user to run commands"
    fi

    if command -v brew > /dev/null; then
        brew services start postgresql
    elif command -v service > /dev/null; then
        service postgresql start
    elif command -v systemctl > /dev/null; then
        $SUDO systemctl start postgresql.service
    else
        print_info "Unable to start postgres. Please start manually"
    fi
}

init_start_redis () {

    if command -v brew > /dev/null; then
        brew services start redis
    elif command -v service > /dev/null; then
        service redis-server start
        service redis start
    elif command -v systemctl > /dev/null; then
        $SUDO systemctl start redis.service
        $SUDO systemctl start redis-server.service
    else
        print_info "Unable to start redis. Please start manually"
    fi
}

print_info ${PRE_INSTALL_MSG}

if prompt "Would you like to continue"; then
    err "Aborted by user"
fi

# verify rust installation and version
if command -v cargo > /dev/null; then

    print_info "\"cargo\" found. Verifying rustc version"

    need_cmd rustc

    RUST_VERSION=`rustc -V | cut -d " " -f 2`

    _HAVE_VERSION=`ver ${RUST_VERSION}`
    _NEED_VERSION=`ver ${RUST_MSRV}`

    print_info "Found rust version \"${RUST_VERSION}\". MSRV is \"${RUST_MSRV}\""

    if [[ ${_HAVE_VERSION} -lt ${_NEED_VERSION} ]] ; then

        if command -v rustup > /dev/null; then
            print_info "found rustup. Trying to install ${RUST_MSRV} version..."
            rustup install "${RUST_MSRV}"
        else
            print_info "Couldn't find \"rustup\". ***needs upgrade***, but skipping..."
        fi
    else
        print_info "Skipping update"
    fi
else
    print_info "\"cargo\" command not found..."
    if ! prompt "Would you like to install \"rust\" using \"rustup\""
    then
        print_info "Installing \"rust\" through \"rustup\""
        need_cmd curl
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
        # add to path
        . "$HOME/.cargo/env"
        need_cmd rustc
    else
        print_info "Rust installation was aborted"
        err "This application needs \"rust\""
    fi
fi

# Dependency checks
print_info "checking for dependencies"

install_dep () {
    $INSTALL_CMD $*
}

if [[ ! -x "`command -v psql`" ]] || [[ ! -x "`command -v redis-server`" ]] ; then
    print_info "Missing dependencies. Trying to install"

    # java has an apt which seems to mess up when we look for apt
    if command -v apt-get > /dev/null; then
        INSTALL_CMD="$SUDO apt-get install -y"
    elif command -v yum > /dev/null; then
        INSTALL_CMD="$SUDO yum -y install"
    elif command -v pacman > /dev/null; then
        INSTALL_CMD="$SUDO pacman -S"
    elif command -v brew > /dev/null; then
        INSTALL_CMD="brew install"
    else
        err "Unable to identify the package manager"
    fi

    if ! command -v psql > /dev/null
    then
       install_dep postgresql
       install_dep postgresql-contrib # not needed for macos?
       install_dep postgresql-devel # needed for diesel_cli in some linux distributions
       install_dep postgresql-libs # needed for diesel_cli in some linux distributions
       init_start_postgres # installing libpq messes with initdb creating two copies. better to run it better libpq.
       install_dep libpq-dev || install_dep libpq
    else
        print_info "Postgres found. skipping..."
    fi

    if ! command -v redis-server > /dev/null
    then
        # the package names differ. Need better way to identify correct package name
       install_dep redis || install_dep redis
       init_start_redis
    else
        print_info "Redis found. skipping..."
    fi

else
    print_info "No missing dependency."
fi

print_info "Proceeding to setup Database"
# install diesel cli
need_cmd cargo
if ! command -v diesel > /dev/null; then
    print_info "diesel_cli not found. Installing..."
    cargo install diesel_cli --no-default-features --features "postgres"
else
    print_info "diesel_cli found."
fi

# DB setup
need_cmd psql
need_cmd diesel
read -p "Enter name for database [default: ${_DB_NAME}]: " DB_NAME
DB_NAME=${DB_NAME:-$_DB_NAME}

read -p "Enter username for ${DB_NAME} [default: ${_DB_USER}]: " DB_USER
DB_USER=${DB_USER:-$_DB_USER}

read -sp "Enter password for ${DB_USER} [default: ${_DB_PASS}]: " DB_PASS
DB_PASS=${DB_PASS:-$_DB_PASS}

print_info ""

print_info "Creating DB \"${DB_NAME}\" and user \"${DB_USER}\" with provided password"


if [[ "$OSTYPE" == "darwin"* ]]; then
    psql -e -U postgres -c "CREATE DATABASE ${DB_NAME};"
    psql -e -U postgres -c "CREATE USER ${DB_USER} WITH PASSWORD '${DB_PASS}';"
    psql -e -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE ${DB_NAME} to ${DB_USER};"
elif command -v su > /dev/null; then
    $SUDO su -c "psql -e -c \"CREATE DATABASE ${DB_NAME};\"" postgres
    $SUDO su -c "psql -e -c \"CREATE USER ${DB_USER} WITH PASSWORD '${DB_PASS}';\"" postgres
    $SUDO su -c "psql -e -c \"GRANT ALL PRIVILEGES ON DATABASE ${DB_NAME} to ${DB_USER};\"" postgres
elif command -v sudo > /dev/null; then
    sudo -u postgres psql -e -c "CREATE DATABASE ${DB_NAME};"
    sudo -u postgres psql -e -U postgres -c "CREATE USER ${DB_USER} WITH PASSWORD '${DB_PASS}';"
    sudo -u postgres psql -e -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE ${DB_NAME} to ${DB_USER};"
else
    err "Don't know how to switch to postgres user to run commands"
fi

# run migrations
print_info "Running migrations"
MIGRATION_CMD="diesel migration --database-url postgres://${DB_USER}:${DB_PASS}@localhost:5432/${DB_NAME} run"

if [[ -d "migrations" ]]; then
    $MIGRATION_CMD
else
    print_info "\"migrations\" directory not found. Please clone the repo/run the below command in the application directory"
    print_info "CMD \"${MIGRATION_CMD}\""
fi

print_info "${POST_INSTALL_MSG}"
