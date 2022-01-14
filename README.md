# stats.gallery Sponsored Badges Smart Contract

This smart contract allows anyone to submit a proposal for a certain action with an attached deposit. These are called "sponsorship proposals," and the attached action is called an "action request." The owner of the smart contract has the ability to accept or reject a proposal within its designated validity period ("duration"). Until the proposal is accepted or rejected ("resolved"), the original author of the proposal may rescind it, receiving their deposit back as well.

The sponsorship and ownership parts of the contract are cleanly separated from the stats.gallery-specific implementation, allowing for reusability of those features.

# Required Software

* Rust 1.56
* Cargo 1.56
* Node.js 14
* NPM 8

# Build

```txt
$ ./build.sh
```

# Deploy

First, edit `./init-args.js` to suit your use-case. In particular, you probably want to change `owner_id` to an account you control.

## Testnet

```txt
$ ./dev-deploy.sh
```

## Mainnet

```txt
$ ./deploy.sh
```

# Usage

See [`/example-proposals`](/example-proposals) for example argument JSON.

* An author wants to propose a badge, so they call `spo_submit(submission)` to submit a proposal.
* An author wants to rescind a badge proposal, so they call `spo_rescind(id)` with the ID of the proposal they wish to rescind.
* Someone wants to view a proposal, so they call `spo_get_proposal(id)` with the ID of the proposal they wish to view.
* The owner wants to reject a proposal, so they call `spo_reject(id)` with the ID of the proposal they wish to reject.
* The owner wants to accept a proposal, so they call `spo_accept(id)` with the ID of the proposal they wish to accept.
* The owner wants to transfer ownership of the contract, so they call `own_propose_owner(account_id)` with the ID of the account they wish to nominate for owner.
* A proposed owner wishes to accept ownership of a contract, so they call `own_accept_owner()` and ownership is transferred to the proposed account.

If you wish to explore and easily interact with this contract, I recommend you deploy it to testnet, and then visit the [stats.gallery contract page](https://stats.gallery/testnet/dev-1642129686546-74039727190323/contract) for it (be sure to input the account ID of *your* deployment, not the sample).

# Authors

* Jacob Lindahl <jacob@near.foundation> [@sudo_build](https://twitter.com/sudo_build)

# License

GPL-3.0-only
