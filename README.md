# Substrate Naming Pallet

This repository contains a **naming pallet** for
[Substrate](https://substrate.dev/)-based blockchains.  Adding this module
implements **[Namecoin-like](https://www.namecoin.org/) functionality**.

## Overview

The *names pallet* implements a **general key/value storage** system for
a Substrate-based blockchain (in the spirit of
[Namecoin](https://www.namecoin.org/)).  The main feature it adds to
a blockchain is a database mapping globally unique *names* to *values*.

What exactly names and values are is up to the developer and can be configured
through the pallet's `Trait`.  In the case of Namecoin, both names and
values are arbitrary byte strings (i.e. `Vec<u8>`).  This is likely the
most useful definition for a lot of applications; however, it may be useful
also to have e.g. the values be custom structs with application-specific data.
Developers using the names module can define custom business logic
that determines what names and values are valid.

As in Namecoin, registrations of names are done on a *first come, first serve*
basis.  This means that any user of the blockchain can register a name that
is not yet in use.  Once registered, only the current *owner* of a name
(which is an account on the blockchain) can associate values to a name.
The owner can also *transfer* a name, which means that the owner account
can be changed.  However, *everyone* on the network can *read* the current
database.  Due to these rules, values in the name database are automatically
authenticated by the name's owner and can be trusted by everyone reading them.

Name operations (creating a name and/or updating an existing name) can
incur a *name fee* (in addition to the blockchain's ordinary transaction
fees).  This can be used as a measure to prevent spam.  The blockchain's
developer can configure what operations cost how much, and also what should
happen with the fees.  For instance, fees could be burnt, sent to the
developer's address or distributed to miners.

Finally, the blockchain can be configured so that name registrations *expire*
if the name is not updated for a certain number of blocks.  This has two
effects:  First, it ensures that names whose owners have lost their keys
will eventually become available again and are not lost permanently.
And second, together with fees for updates, this adds a running (as opposed
to one-time) cost for name ownership, thus discouraging squatting and
encouraging economic usage of names.

## Getting Started

In addition to the [names pallet
itself](https://github.com/xaya/substrate-names/tree/master/names),
this repository also contains a full [Substrate example
node](https://github.com/xaya/substrate-names/tree/master/node)
that uses it and a
[minimal frontend](https://github.com/xaya/substrate-names/tree/master/frontend)
for trying it out.

The first step for getting started is to install the prerequisites for
running Substrate and the frontend.  For this, just follow the
corresponding steps in the Substrate [Proof-of-Existence
tutorial](https://substrate.dev/docs/en/next/tutorials/creating-your-first-substrate-chain/setup#prerequisites).

### Running the Blockchain

Next, build and start the example node:

    cd node
    cargo build --release
    target/release/node --dev

When all goes well, the blockchain node (with names functionality) will
be started and the console will show blocks being produced.

### Starting the Frontend

To start the frontend:

    cd frontend
    yarn install
    yarn start

This will start a local webserver on
[http://localhost:8000/](http://localhost:8000/), and also
open a browser window showing the frontend.

### Testing Name Operations

In the frontend, name operations can be tested using the *Extrinsics* section
(with module set to `names`).  The function `update` can be used to
register a name (when it does not yet exist) or change its value
(if it exists already).  The first input is the name (as string), and
the second input is the value to update it to (also as string).

Similarly, the `transfer` function will change a name's owner.  The first
input is the name and the second input the address of the new owner account.

When name operations are included in blocks, the *Events* section will list
those events.  Also, the current state of the name database can be
read with the *Chain State* section (module and item `names`, the input
is the name as string).

### Configuration of the Example Blockchain

The example node has the following rules configured, which are sufficient
to test all essential features of the names module:

- Names have to be at least two bytes long (single letter names as well
  as the empty string are not allowed).
- Registrations of new names cost 1'000 coins, updates 100.  Those fees
  are burnt.
- Names with up to three characters in length will expire after 10 blocks.
  Longer names will never expire.
