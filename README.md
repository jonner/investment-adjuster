# investment-adjuster
This is a very simple program that takes as input a file describing an
investment portfolio (currently only Fidelity CSV format is supported). It
compares the holdings in the portfolio to a target allocation configuration
defined in a yaml file. It calculates the necessary amounts to buy and sell in
order to bring the portfolio into alignment with the target allocation.

## Configuration
Create a `target.yml` file in your application config directory by running
`investment-adjuster edit`. In that configuration file, you should specify the
target fund allocations for your accounts. For example:

```yaml
- AccountNumber: "123456789"
  CashSweep:
    Symbol: "FZFXX"
    Minimum: 1000
  Targets:
    "FXNAX": 25
    "FSKAX": 45
    "FTIHX": 30
```

You can specify a minimum dollar value that you want to leave as cash in your
cash sweep, and then targets for the other investments as percentages.

The target file can contain multiple account definitions, but they should each
have unique `AccountNumber`s.

## Basic Usage
Then download your account data from the fidelity website. Go to the 'Positions'
tab in your account, and click the 3 dots icon in the top right of the table and
select 'Download'. Then run the application against this csv file:
```
$ investment-adjuster adjust portfolio.csv

Account 123456789: Retirement Account
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
value, so it allocates the excess $1000.00 to buy new investments. It uses the
value of that excess cash and the total value of the existing investments, and
tries to allocate them according to the percentages given in the configuration
file. To achieve the desired allocation percentages, you need to sell some
investments that are over-allocated and buy some that are under-allocated.

If the cash sweep is already below the specified minimum, it will try to sell
investments to get back up to that minimum value. For example, if we had the
minimum cash value set to $3000.00:

```
$ investment-adjuster adjust portfolio.csv

Account 123456789: Retirement Account
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
