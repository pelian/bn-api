# Big Neon API

[![Build Status](https://drone.metalworks.tarilabs.com/api/badges/big-neon/bn-api/status.svg)](https://drone.metalworks.tarilabs.com/big-neon/bn-api)
[![Docker Repository on Quay](https://quay.io/repository/tarilabs/bn-api/status "Docker Repository on Quay")](https://quay.io/repository/tarilabs/bn-api)

This is the Big Neon API repository, part of the Big Neon ticketing system.

# Overall project architecture

Big Neon is a multi-tiered micro-services architecture for selling and managing tickets. The software system is made up 
of multiple components. As such, the code is split across multiple repositories. To get an overall picture of how 
everything fits together have a look at the [docs repository]( https://github.com/big-neon/docs.git)

# Building this project from source

To download and build this project, 

1. Clone the source

        git clone https://github.com/big-neon/bn-api.git
    
1. Compile
        
        cargo build
        
# Configuring the local development environment

This code inter-operates with code in several other repositories. To simplify the management of your local environment,
we've created the [Big Neon repository](https://github.com/big-neon/bigneon) that uses [Docker](https://docker.org) to
set up and configure your local development environment.

See the [README](https://github.com/big-neon/bigneon/blob/master/README.md) for that repo for more details.

# Setting up Facebook login
Facebook login is optional. If you would like to enable it, you will need to get
an `app id` via the Facebook Developer page.

On the Facebook Developer page, you will also need to set the `Valid  OAuth Redirect URIs`.
