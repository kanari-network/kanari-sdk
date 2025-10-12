
"use client";

import axios from "axios";

// API endpoint - prefer env var, fall back to previous hardcoded URL
export const API_URL: string = process.env.NEXT_PUBLIC_API_URL || "http://192.168.1.101:30030";

type RpcParams = any;

// Domain types
export interface Account {
	address: string;
	balance: number;
	balance_formatted: string;
	transaction_count: number;
	is_contract: boolean;
	account_type?: string;
}

export interface Transaction {
	id?: string;
	sender: string;
	receiver: string;
	amount: number;
	amount_formatted?: string;
	gas_fee?: number;
	gas_fee_formatted?: string;
	timestamp?: number;
	block_index?: number;
	hash?: string;
}

export interface AccountDetails {
	address: string;
	balance: number;
	balance_formatted: string;
	account_type?: string;
	is_contract?: boolean;
	transaction_count?: number;
	transactions?: Transaction[];
	code?: string;
}

export interface TransactionSearchResult {
	transactions?: Transaction[];
	total_count?: number;
}

export interface GasFeeInfo {
	current_gas_price?: number;
	current_gas_price_formatted?: string;
	network_congestion?: string;
	gas_collection_address?: string;
}

export interface StakingStats {
	total_staked_amount?: number;
	total_staked_amount_formatted?: string;
	total_validators?: number;
	total_nodes?: number;
	average_reward_rate?: number;
	latest_rewards_distributed?: number;
	latest_rewards_distributed_formatted?: string;
}

export interface StakingInfo {
	address: string;
	is_staking: boolean;
	minimum_staking_amount?: number;
	minimum_staking_formatted?: string;
	minimum_validator_amount?: number;
	minimum_validator_formatted?: string;
	staked_amount?: number;
	staked_amount_formatted?: string;
	is_validator?: boolean;
	rewards_earned?: number;
	rewards_earned_formatted?: string;
	stake_date?: number;
	unlock_date?: number;
	status?: string;
}

// Blocks and chain status
export interface Block {
	index: number;
	hash: string;
	prev_hash: string;
	timestamp: number;
	datetime?: string;
	miner?: string;
	transactions: Transaction[];
	transaction_count: number;
	tokens_minted?: number;
}

export interface BlockchainStatus {
	chain_id: string;
	block_height?: number;
	block_count?: number;
	latest_block?: {
		index: number;
		hash: string;
		timestamp: number;
		transactions: number;
		miner?: string;
	};
	total_transactions?: number;
	genesis_timestamp?: number;
	totalSupply?: number;
}

export async function getAllBlocks(): Promise<{ blocks?: Block[]; block_count?: number }> {
	return rpcPost("get_all_blocks", []);
}

export async function getBlockchainStatus(): Promise<BlockchainStatus> {
	return rpcPost("blockchain_status", []);
}

async function rpcPost(method: string, params: RpcParams = []): Promise<any> {
	const res = await axios.post(
		API_URL,
		{
			jsonrpc: "2.0",
			method,
			params,
			id: 1,
		},
		{
			headers: { "Content-Type": "application/json" },
		}
	);

	if (res.data.error) {
		throw new Error(res.data.error.message || "RPC error");
	}

	return res.data.result;
}

export async function listAccounts(): Promise<{ accounts?: Account[] }> {
	return rpcPost("list_accounts", []);
}

export async function getAccountDetails(address: string): Promise<AccountDetails> {
	return rpcPost("get_account_details", { address });
}

export async function getTransactionById(txid: string): Promise<Transaction> {
	return rpcPost("get_transaction_by_id", txid);
}

export async function searchTransactions(address: string, limit = 10, offset = 0): Promise<TransactionSearchResult> {
	return rpcPost("search_transactions", { address, limit, offset });
}

export async function getGasFeeInfo(): Promise<GasFeeInfo> {
	return rpcPost("get_gas_fee_info", []);
}

export async function getStakingStats(): Promise<StakingStats> {
	return rpcPost("get_staking_stats", []);
}

export async function getStakingInfo(address: string): Promise<StakingInfo> {
	return rpcPost("get_staking_info", [address]);
}
