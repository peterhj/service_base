# service_base

This is a coordination point and common toolkit for building
out a service architecture in a distributed system based on
"plain old networking":

- services are hosted at ports on 127.0.0.1
- clients _query_ services
- services _reply_ to clients
- message-passing is just bytes over a sequential stream
  (currently just TCP)
- typed messages (`enum Msg`) are a thin abstraction over
  JSON serialization/deserialization

Address tunneling between hosts is currently assumed to be
out-of-band.
Please also see the [devops](../devops) repository for
related scripts.

## Usage notes

The `enum Msg` type is meant to be extended with any and all
domain-specific message types that may be useful to the
implementing services.
(Thus, `service_base` is a _coordination point_ between
services.)
