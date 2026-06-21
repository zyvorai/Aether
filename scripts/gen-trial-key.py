#!/usr/bin/env python3
"""
Zyvor Sales — Trial Licence Key Generator for Aether

Usage:
  python3 scripts/gen-trial-key.py --who "Acme Corp" [--days 30]

The generated key is sent to the customer. They set it via:
  export AETHER_LICENSE_KEY="<key>"
  # or place in ~/.config/aether/license.key

KEEP THIS SCRIPT AND THE HMAC_SECRET CONFIDENTIAL.
"""

import argparse
import base64
import datetime
import hmac
import hashlib
import json

# Must match license.rs
HMAC_SECRET = b"zyvor-aether-trial-v1-3b5e7f1a9c2d4e6f"
PRODUCT     = "aether"

def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

def gen_key(licensee: str, days: int, issued_date: datetime.date | None = None) -> str:
    today  = issued_date or datetime.date.today()
    expiry = today + datetime.timedelta(days=days)
    payload = json.dumps({
        "p":   PRODUCT,
        "iss": today.isoformat(),
        "exp": expiry.isoformat(),
        "who": licensee,
    }, separators=(",", ":")).encode()
    payload_b64 = b64url(payload)
    sig = hmac.new(HMAC_SECRET, payload_b64.encode(), hashlib.sha256).digest()
    return f"{payload_b64}.{b64url(sig)}"

def main():
    ap = argparse.ArgumentParser(description="Generate an Aether trial licence key")
    ap.add_argument("--who",   required=True, help="Licensee company/name")
    ap.add_argument("--days",  type=int, default=30, help="Trial duration in days (default: 30)")
    ap.add_argument("--issued", help="Override issue date (YYYY-MM-DD)")
    args = ap.parse_args()

    issued = None
    if args.issued:
        issued = datetime.date.fromisoformat(args.issued)

    key = gen_key(args.who, args.days, issued)
    today  = issued or datetime.date.today()
    expiry = today + datetime.timedelta(days=args.days)

    print(f"\n  Aether Trial Licence Key")
    print(f"  Licensee : {args.who}")
    print(f"  Issued   : {today}")
    print(f"  Expires  : {expiry}  ({args.days} days)")
    print(f"\n  KEY:\n  {key}\n")
    print("  Delivery instructions:")
    print("  1. Email the key to the customer.")
    print("  2. Customer sets: export AETHER_LICENSE_KEY=\"<key>\"")
    print("     OR places in: ~/.config/aether/license.key")
    print("  3. They can verify with: aether --version\n")

if __name__ == "__main__":
    main()
