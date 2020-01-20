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
