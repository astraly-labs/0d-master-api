#!/bin/bash

echo "🔄 Testing Master API vs Extended Vault API Endpoints"
echo "====================================================="

# Function to clean HTML responses
clean_response() {
    local response="$1"
    if [[ "$response" == *"<html>"* ]] || [[ "$response" == *"<head>"* ]]; then
        if [[ "$response" == *"502 Bad Gateway"* ]]; then
            echo "502 Bad Gateway"
        elif [[ "$response" == *"503 Service Temporarily Unavailable"* ]]; then
            echo "503 Service Unavailable"
        elif [[ "$response" == *"404 Not Found"* ]]; then
            echo "404 Not Found"
        else
            echo "HTML Error Response"
        fi
    else
        echo "$response"
    fi
}

echo ""
echo "❤️ HEALTH CHECK"
echo "Master API:"
master_health=$(curl -s "http://localhost:4242/health")
clean_response "$master_health"
echo "Extended API:"
extended_health=$(curl -s "https://extended-vault-api.production.pragma.build/health")
clean_response "$extended_health"
echo ""

sleep 1

echo ""
echo "📊 VAULT STATS"
echo "Master API:"
master_stats=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/stats")
clean_response "$master_stats"
echo "Extended API:"
extended_stats=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/stats")
clean_response "$extended_stats"
echo ""

sleep 1

echo ""
echo "📈 VAULT KPIs"
echo "Master API:"
master_kpis=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/kpis")
clean_response "$master_kpis"
echo "Extended API:"
extended_kpis=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/kpis?timeframe=all")
clean_response "$extended_kpis"
echo ""

sleep 1

echo ""
echo "💰 VAULT APR SUMMARY"
echo "Master API:"
master_apr=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/apr/summary")
clean_response "$master_apr"
echo "Extended API:"
extended_apr=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/apr/summary?basis=30d")
clean_response "$extended_apr"
echo ""

sleep 1

echo ""
echo "📊 VAULT APR SERIES"
echo "Master API:"
master_series=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/apr/series")
clean_response "$master_series"
echo "Extended API:"
extended_series=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/apr/series?timeframe=30d")
clean_response "$extended_series"
echo ""

sleep 1

echo ""
echo "🎯 VAULT COMPOSITION"
echo "Master API:"
master_comp=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/composition")
clean_response "$master_comp"
echo "Extended API:"
extended_comp=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/composition")
clean_response "$extended_comp"
echo ""

sleep 1

echo ""
echo "💧 VAULT LIQUIDITY"
echo "Master API:"
master_liq=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/liquidity")
clean_response "$master_liq"
echo "Extended API:"
extended_liq=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/liquidity")
clean_response "$extended_liq"
echo ""

sleep 1

echo ""
echo "📈 VAULT NAV LATEST"
echo "Master API:"
master_nav=$(curl -s "http://localhost:4242/v1/vaults/carry-trade-vault-usdc/nav/latest")
clean_response "$master_nav"
echo "Extended API:"
extended_nav=$(curl -s "https://extended-vault-api.production.pragma.build/v1/master/nav/latest")
clean_response "$extended_nav"
echo ""

echo ""
echo "🎯 Test completed!"