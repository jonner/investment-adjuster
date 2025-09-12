# investment-adjuster
This is a very simple program that takes as input a file describing an
investment portfolio (currently only Fidelity CSV format is supported). It
compares the holdings in the portfolio to a target allocation configuration
defined in a yaml file. It calculates the changes necessary amounts to buy and
sell in order to bring the portfolio into alignment with the target allocation.

## Configuration
Create a `target.yml` file in your application config directory (or run
the application without specifying a target file and it should give you the
expected path to the file in its error message) that specifies the target fund
allocations for your account. For example:

```yaml
AccountNumber: "X12345678"
CorePosition:
  Symbol: "FZFXX"
  Minimum: 2000
Allocations:
  "FXNAX": 25
  "FSKAX": 45
  "FTIHX": 30
```

You can specify a minimum dollar value that you want to leave as 'cash' in your
core position, and then percentages for the other investments as percentages.

## Basic Usage
Then download your account data from the fidelity website. Go to the 'Positions'
tab in your account, and click the 3 dots icon in the top right of the table and
select 'Download'. Then run the application against this csv file:
```
$ investment-adjuster ~/Downloads/Portfolio_Positions_Sep-12-2025.csv

Account X12345678
╭────────┬──────────┬─────────┬────────┬──────────┬─────────┬─────────╮
│ Symbol │    Value │ Percent │ Target │   Retain │    Sell │     Buy │
├────────┼──────────┼─────────┼────────┼──────────┼─────────┼─────────┤
│  FZFXX │ $2805.50 │   37.8% │        │ $2000.00 │ $805.50 │         │
│  FXNAX │ $1593.00 │   21.5% │  25.0% │          │ $236.82 │         │
│  FSKAX │ $2182.20 │   29.4% │  45.0% │          │         │ $258.92 │
│  FTIHX │  $844.00 │   11.4% │  30.0% │          │         │ $783.41 │
╰────────┴──────────┴─────────┴────────┴──────────┴─────────┴─────────╯
[ To change allocation targets, edit the file "/home/user/.config/investment-adjuster/target.yml" ]
```

The application noticed that the core position exceeded the minimum value, so
it allocates the excess $805.50 to buy new investments. It uses the value of
that excess cash and the total value of the existing investments, and tries to
allocate them according to the percentages given in the `targets.yml` file. To
achieve the desired allocation percentages, you need to sell some investments
that are over-allocated and buy some that are under-allocated.

If the core position is already below the specified minimum, it will try to sell
investments to get back up to that minimum value:
```
$ investment-adjuster ~/Downloads/Portfolio_Positions_Sep-12-2025.csv

Account X12345678
╭────────┬──────────┬─────────┬────────┬──────────┬─────────┬─────────╮
│ Symbol │    Value │ Percent │ Target │   Retain │    Sell │     Buy │
├────────┼──────────┼─────────┼────────┼──────────┼─────────┼─────────┤
│  FZFXX │ $1805.50 │   28.1% │        │ $2000.00 │         │ $194.50 │
│  FXNAX │ $1593.00 │   24.8% │  25.0% │          │ $486.82 │         │
│  FSKAX │ $2182.20 │   34.0% │  45.0% │          │ $191.08 │         │
│  FTIHX │  $844.00 │   13.1% │  30.0% │          │         │ $483.41 │
╰────────┴──────────┴─────────┴────────┴──────────┴─────────┴─────────╯
[ To change allocation targets, edit the file "/home/user/.config/investment-adjuster/target.yml" ]
```

**NOTE**: The `Percent` column displays the percent of the investment as
a percentage of **all** money in the account. But the target allocation
configuration applies to the total account value after retaining the minimum
amount in the core position. So after re-allocating, the resulting percentage
values displayed in the table won't necessarily match the percentage values
specified in the config file.
