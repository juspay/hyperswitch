# Running scenario-mix on EC2

Guide for running this load test from an EC2 instance instead of locally —
useful once the load generator itself becomes the bottleneck (local CPU
maxing out before the target service does).

## 1. Launch the EC2 instance

- **AMI**: Amazon Linux 2023 or Ubuntu 22.04 (both covered below).
- **Instance type**: k6 is CPU-bound generating load (JS VU execution + TLS),
  not memory-bound. `config.json`'s `target_rps: 200` with up to ~6
  requests/iteration and `max_vus_multiplier: 10` across 4 scenario entries
  means headroom for a couple thousand VUs. Start with a compute-optimized
  box:
  - `c6i.2xlarge` (8 vCPU / 16GB) as a baseline.
  - `c6i.4xlarge` (16 vCPU) if pushing `target_rps` well past 200 for
    breakpoint testing.
- **Storage**: default 20GB gp3 is plenty (script + reports are tiny).
- **Security group**: inbound SSH (22) from your IP only. No inbound needed
  otherwise — k6 only makes outbound calls to `services.router`/
  `modular_pm`. Default allow-all outbound is fine.
- **Key pair**: create/reuse one for SSH.

## 2. Install k6 on the instance

SSH in, then:

**Amazon Linux 2023:**

```bash
sudo dnf install -y https://dl.k6.io/rpm/repo.rpm 2>/dev/null || \
  (curl -s https://dl.k6.io/key.gpg | sudo rpm --import - && \
   sudo tee /etc/yum.repos.d/k6.repo <<'EOF'
[k6]
name=k6
baseurl=https://dl.k6.io/rpm
enabled=1
gpgcheck=1
gpgkey=https://dl.k6.io/key.gpg
EOF
   sudo dnf install -y k6)
```

**Ubuntu:**

```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update && sudo apt-get install -y k6
```

Verify: `k6 version`

## 3. Copy the required files

`scenario-mix.js` is fully self-contained (no imports from `../helper/`), so
only `scenario-mix/` needs to go over:

```bash
# From your local loadtest/ dir
scp -i <your-key.pem> -r k6/scenario-mix ec2-user@<EC2_IP>:~/loadtest-k6/
```

(use `ubuntu@` instead of `ec2-user@` for Ubuntu AMIs)

That copies `config.json` too — which currently has live merchant
credentials in it. Treat that transfer/host like you'd treat any secret:
don't leave the box open to others, and consider whether that config should
ever have been committed to git in the first place (it looks like it was,
in `fe2af8af44`). Not blocking for the load test, but worth cleanup
separately.

`scenario-mix.js` writes `SUMMARY_OUTPUT` into an `output/` directory by
default (`OUTPUT_DIR`, see README "Collecting everything into one output
directory") — it must exist before the run, since k6 can't create missing
directories itself:

```bash
ssh -i <your-key.pem> ec2-user@<EC2_IP>
mkdir -p ~/loadtest-k6/scenario-mix/output
```

## 4. Run it

```bash
ssh -i <your-key.pem> ec2-user@<EC2_IP>
cd ~/loadtest-k6/scenario-mix
SUMMARY_OUTPUT=summary.json k6 run -q \
  --console-output=output/failures.log \
  --out 'web-dashboard=export=output/timeseries.html&period=2s' \
  scenario-mix.js
```

That writes `output/summary.json` (default `OUTPUT_DIR`), plus
`output/failures.log` and `output/timeseries.html` (explicit paths, since
`--console-output`/`--out` aren't affected by `OUTPUT_DIR` — see README)
into the directory created above.

Watch `dropped_iterations` in the output — if non-zero, the box still can't
sustain the requested rate; bump `pre_allocated_vus_multiplier`/
`max_vus_multiplier` in `config.json`, or move up an instance size, before
trusting the numbers.

Want to watch it live instead of only waiting for the export? The dashboard
is already live on port 5665 while the run above executes (`export=` just
also saves a copy at the end — it doesn't disable the live server, and `-q`
only silences the terminal progress bar, not the dashboard) — see *Viewing
the web dashboard live from your machine* below to reach it from your
browser.

## 5. Export the results back to your machine

```bash
scp -i <your-key.pem> -r ec2-user@<EC2_IP>:~/loadtest-k6/scenario-mix/output ./
```

Open `output/timeseries.html` locally in a browser — it's self-contained (no
network calls needed to render) and shows throughput/latency evolving over
the run, not just end-of-run aggregates.

## Viewing the web dashboard live from your machine

If you'd rather watch the dashboard update in real time instead of waiting
for the export, tunnel the instance's dashboard port over SSH rather than
opening it to the internet (the dashboard has no authentication):

```bash
ssh -i <your-key.pem> -L 5665:localhost:5665 ec2-user@<EC2_IP>
```

Then, in that same session (or another one), run k6 with `--out
web-dashboard` (no `export=` needed for the live view) and open
`http://localhost:5665` in your local browser — it's tunneled through SSH,
so port 5665 never needs to be reachable from outside the instance.

## 6. Run it detached (optional, useful for long soak tests)

If you want the test to survive an SSH disconnect:

```bash
SUMMARY_OUTPUT=summary.json nohup k6 run -q \
  --console-output=output/failures.log \
  --out 'web-dashboard=export=output/timeseries.html&period=2s' \
  scenario-mix.js > run.log 2>&1 &
```

or use `tmux`/`screen` on the instance.

## Tuning note

If doing breakpoint/ramp testing (`load.phases`) to find max RPS, watch the
EC2 instance's own CPU (`top`/`htop`) during the run — a saturated *load
generator* looks identical to a saturated *target*, so confirm the EC2 box
itself isn't the bottleneck before concluding the router/PM service
capacity.
