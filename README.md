# driftfix
This is a very simple program that helps you maintain target allocations within
an investment portfolio. It compares the holdings in the portfolio to a target
allocation configuration defined in a yaml file. It calculates the necessary
actions you need to take (buy or sell) in order to bring the portfolio into
alignment with the target allocations.

## Configuration
To get started, create a target allocation configuration. Each account is
configured separately. To get started, run `driftfix configure`. An editor will
be opened to edit your configuration. If this is the first time you've run the
`configure` command, it will present an example configuration to help you with
the configuration syntax. For this tutorial, we'll assume we have the following
configuration.

```yaml
- AccountId: "123456789"
  CashSweep:
    Symbol: "FZFXX"
    Minimum: 1000.0
  Targets:
    "FXNAX": 25.0
    "FSKAX": 45.0
    "FTIHX": 30.0
```

You can specify a minimum dollar value that you want to leave as cash in your
cash sweep, and then specify target allocation percentages for other investments.

The target file may define targets for multiple accounts, but they must each
have unique `AccountId`s.

## Import your balance data
Download (or create) a file containing your account balances. Currently,
the CSV portfolio format that can be downloaded from Fidelity is the only
fully supported file format. To download this file from Fidelity, go to the
'Positions' tab in your account, and click the 3 dots icon in the top right of
the table and select 'Download'. Then import this data into the application by
running `driftfix data add <FILENAME>`.

Any time you want to update your account balances, simply download a new file
and re-run the same command with the new file. It will update all account
balances contained in the file.

## Plan 
After some time, some investments will perform better, and some will perform
worse, and your investment allocations will drift from your configured target.
To calculate what actions you need to take to re-balance your investments,
simply run `driftfix plan`. You will get output similar to the following:
```
$ driftfix plan

Retirement Account
Account ID: 123456789
Total balance: $13817.00
╭────────┬──────────┬─────────┬────────┬──────────┬──────────┬──────────╮
│ Symbol │    Value │ Percent │ Target │     Sell │      Buy │   Result │
├────────┼──────────┼─────────┼────────┼──────────┼──────────┼──────────┤
│  FZFXX │ $2000.00 │   14.5% │   0.0% │ $1000.00 │          │ $1000.00 │
│  FSKAX │ $9397.50 │   68.0% │  45.0% │ $3629.85 │          │ $5767.65 │
│  FXNAX │ $1567.50 │   11.3% │  25.0% │          │ $1636.75 │ $3204.25 │
│  FTIHX │  $852.00 │    6.2% │  30.0% │          │ $2993.10 │ $3845.10 │
╰────────┴──────────┴─────────┴────────┴──────────┴──────────┴──────────╯
```

The application noticed that the cash sweep exceeded the minimum configured
value, so it advises you to use the excess $1000.00 to buy new investments.
Using the value of that excess cash and the total value of the existing
investments, it recommends selling investments that exceed the target allocation
and buying investments that are below the target.

If the cash sweep is already below the specified minimum, it will advise you to
sell enough investments to get back up to that minimum value. For example, if we
had set the minimum cash value to $3000.00, it would suggest something like:

```
$ driftfix plan

Retirement Account
Account ID: 123456789
Total balance: $13817.00
╭────────┬──────────┬─────────┬────────┬──────────┬──────────┬──────────╮
│ Symbol │    Value │ Percent │ Target │     Sell │      Buy │   Result │
├────────┼──────────┼─────────┼────────┼──────────┼──────────┼──────────┤
│  FZFXX │ $2000.00 │   14.5% │   0.0% │          │ $1000.00 │ $3000.00 │
│  FSKAX │ $9397.50 │   68.0% │  45.0% │ $4529.85 │          │ $4867.65 │
│  FXNAX │ $1567.50 │   11.3% │  25.0% │          │ $1136.75 │ $2704.25 │
│  FTIHX │  $852.00 │    6.2% │  30.0% │          │ $2393.10 │ $3245.10 │
╰────────┴──────────┴─────────┴────────┴──────────┴──────────┴──────────╯
```

**NOTE**: The `Percent` column displays the percent of the investment as
a percentage of **all** money in the account. But the target allocation
configuration applies to the total account value after retaining the minimum
amount in the core position. So after re-allocating, the resulting percentage
values displayed in the table won't necessarily match the percentage values
specified in the config file.

## Data management
There are several subcommands under the `data` command that allow you to manage
data that is stored by the application. You can view data, remove data for a
single account, or clear all data stored by the application. See the command
help for more information.
