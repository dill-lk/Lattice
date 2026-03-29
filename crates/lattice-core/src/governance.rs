//! Governance Module - On-chain voting and proposals
//!
//! Allows LAT token holders to vote on protocol changes

use crate::{Address, Amount, BlockHeight, Hash};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proposal ID
pub type ProposalId = u64;

/// Proposal type
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum ProposalType {
    /// Change network parameter
    ParameterChange {
        parameter: String,
        new_value: String,
    },
    /// Protocol upgrade
    ProtocolUpgrade {
        version: String,
        code_hash: Hash,
    },
    /// Treasury spending
    TreasurySpend {
        recipient: Address,
        amount: Amount,
        description: String,
    },
    /// Validator set change
    ValidatorChange {
        validator: Address,
        action: ValidatorAction,
    },
    /// General text proposal
    TextProposal {
        title: String,
        description: String,
    },
}

/// Validator action
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum ValidatorAction {
    Add,
    Remove,
    UpdateStake(Amount),
}

/// Proposal status
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Proposal is active and accepting votes
    Active,
    /// Proposal passed and is in execution queue
    Passed,
    /// Proposal was rejected
    Rejected,
    /// Proposal was executed
    Executed,
    /// Proposal was canceled
    Canceled,
    /// Proposal expired without reaching quorum
    Expired,
}

/// Vote choice
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

/// Individual vote
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Vote {
    /// Voter address
    pub voter: Address,
    /// Vote choice
    pub choice: VoteChoice,
    /// Voting power (based on staked tokens)
    pub voting_power: Amount,
    /// Block height when vote was cast
    pub block_height: BlockHeight,
    /// Timestamp
    pub timestamp: u64,
}

/// Governance proposal
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique proposal ID
    pub id: ProposalId,
    /// Proposer address
    pub proposer: Address,
    /// Proposal type and details
    pub proposal_type: ProposalType,
    /// Proposal description
    pub description: String,
    /// Creation block height
    pub created_at: BlockHeight,
    /// Voting start block
    pub voting_starts: BlockHeight,
    /// Voting end block
    pub voting_ends: BlockHeight,
    /// Execution block (if passed)
    pub execution_block: Option<BlockHeight>,
    /// Current status
    pub status: ProposalStatus,
    /// Votes cast
    pub votes: HashMap<Address, Vote>,
    /// Total yes votes (voting power)
    pub yes_votes: Amount,
    /// Total no votes (voting power)
    pub no_votes: Amount,
    /// Total abstain votes (voting power)
    pub abstain_votes: Amount,
    /// Deposit amount (to prevent spam)
    pub deposit: Amount,
}

impl Proposal {
    /// Create a new proposal
    pub fn new(
        id: ProposalId,
        proposer: Address,
        proposal_type: ProposalType,
        description: String,
        created_at: BlockHeight,
        voting_period: BlockHeight,
        deposit: Amount,
    ) -> Self {
        Self {
            id,
            proposer,
            proposal_type,
            description,
            created_at,
            voting_starts: created_at + 100,  // 100 block delay
            voting_ends: created_at + 100 + voting_period,
            execution_block: None,
            status: ProposalStatus::Active,
            votes: HashMap::new(),
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
            deposit,
        }
    }

    /// Cast a vote
    pub fn vote(&mut self, voter: Address, choice: VoteChoice, voting_power: Amount, current_block: BlockHeight) -> Result<(), GovernanceError> {
        // Check if voting is active
        if current_block < self.voting_starts {
            return Err(GovernanceError::VotingNotStarted);
        }
        if current_block > self.voting_ends {
            return Err(GovernanceError::VotingEnded);
        }
        if self.status != ProposalStatus::Active {
            return Err(GovernanceError::ProposalNotActive);
        }

        // Check if already voted
        if let Some(existing_vote) = self.votes.get(&voter) {
            // Remove old vote counts
            match existing_vote.choice {
                VoteChoice::Yes => self.yes_votes -= existing_vote.voting_power,
                VoteChoice::No => self.no_votes -= existing_vote.voting_power,
                VoteChoice::Abstain => self.abstain_votes -= existing_vote.voting_power,
            }
        }

        // Add new vote
        let vote = Vote {
            voter: voter.clone(),
            choice,
            voting_power,
            block_height: current_block,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        match choice {
            VoteChoice::Yes => self.yes_votes += voting_power,
            VoteChoice::No => self.no_votes += voting_power,
            VoteChoice::Abstain => self.abstain_votes += voting_power,
        }

        self.votes.insert(voter, vote);
        Ok(())
    }

    /// Finalize proposal (after voting ends)
    pub fn finalize(&mut self, governance_config: &GovernanceConfig, total_supply: Amount) {
        let total_votes = self.yes_votes + self.no_votes + self.abstain_votes;
        let quorum_threshold = (total_supply as f64 * governance_config.quorum_percentage) as Amount;
        let approval_threshold = (total_votes as f64 * governance_config.approval_percentage) as Amount;

        // Check quorum
        if total_votes < quorum_threshold {
            self.status = ProposalStatus::Expired;
            return;
        }

        // Check approval
        if self.yes_votes >= approval_threshold {
            self.status = ProposalStatus::Passed;
        } else {
            self.status = ProposalStatus::Rejected;
        }
    }

    /// Get vote counts
    pub fn tally(&self) -> (Amount, Amount, Amount) {
        (self.yes_votes, self.no_votes, self.abstain_votes)
    }

    /// Get participation rate
    pub fn participation_rate(&self, total_supply: Amount) -> f64 {
        let total_votes = self.yes_votes + self.no_votes + self.abstain_votes;
        total_votes as f64 / total_supply as f64
    }

    /// Check if proposal passed
    pub fn passed(&self) -> bool {
        self.status == ProposalStatus::Passed
    }
}

/// Governance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// Minimum deposit to create proposal
    pub min_deposit: Amount,
    /// Voting period in blocks
    pub voting_period: BlockHeight,
    /// Quorum percentage (0.0 to 1.0)
    pub quorum_percentage: f64,
    /// Approval percentage (0.0 to 1.0)
    pub approval_percentage: f64,
    /// Execution delay after passing (blocks)
    pub execution_delay: BlockHeight,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            // 1000 LAT = 1000 * 10^8 Latt (using 8 decimals)
            min_deposit: 100_000_000_000,  // 1000 LAT
            voting_period: 40320,           // ~7 days (15s blocks)
            quorum_percentage: 0.10,        // 10% of supply must vote
            approval_percentage: 0.51,      // 51% of votes must be yes
            execution_delay: 5760,          // ~24 hours
        }
    }
}

/// Governance system
pub struct GovernanceSystem {
    /// Configuration
    config: GovernanceConfig,
    /// All proposals
    proposals: HashMap<ProposalId, Proposal>,
    /// Next proposal ID
    next_proposal_id: ProposalId,
    /// Voting power by address (staked tokens)
    voting_power: HashMap<Address, Amount>,
    /// Total supply of tokens
    total_supply: Amount,
}

impl GovernanceSystem {
    /// Create new governance system
    pub fn new(config: GovernanceConfig, total_supply: Amount) -> Self {
        Self {
            config,
            proposals: HashMap::new(),
            next_proposal_id: 1,
            voting_power: HashMap::new(),
            total_supply,
        }
    }

    /// Update voting power for an address
    pub fn update_voting_power(&mut self, address: Address, power: Amount) {
        self.voting_power.insert(address, power);
    }

    /// Get voting power for an address
    pub fn get_voting_power(&self, address: &Address) -> Amount {
        self.voting_power.get(address).copied().unwrap_or(0)
    }

    /// Create a new proposal
    pub fn create_proposal(
        &mut self,
        proposer: Address,
        proposal_type: ProposalType,
        description: String,
        current_block: BlockHeight,
        deposit: Amount,
    ) -> Result<ProposalId, GovernanceError> {
        // Check deposit
        if deposit < self.config.min_deposit {
            return Err(GovernanceError::InsufficientDeposit);
        }

        // Check proposer has voting power
        let proposer_power = self.get_voting_power(&proposer);
        if proposer_power == 0 {
            return Err(GovernanceError::NoVotingPower);
        }

        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = Proposal::new(
            proposal_id,
            proposer,
            proposal_type,
            description,
            current_block,
            self.config.voting_period,
            deposit,
        );

        self.proposals.insert(proposal_id, proposal);
        Ok(proposal_id)
    }

    /// Cast a vote on a proposal
    pub fn vote(
        &mut self,
        proposal_id: ProposalId,
        voter: Address,
        choice: VoteChoice,
        current_block: BlockHeight,
    ) -> Result<(), GovernanceError> {
        // Get voting power
        let voting_power = self.get_voting_power(&voter);
        if voting_power == 0 {
            return Err(GovernanceError::NoVotingPower);
        }

        // Get proposal
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound)?;

        // Cast vote
        proposal.vote(voter, choice, voting_power, current_block)?;
        Ok(())
    }

    /// Process proposals (finalize ended proposals)
    pub fn process_proposals(&mut self, current_block: BlockHeight) {
        for proposal in self.proposals.values_mut() {
            if proposal.status == ProposalStatus::Active && current_block > proposal.voting_ends {
                proposal.finalize(&self.config, self.total_supply);
                
                // Set execution block if passed
                if proposal.status == ProposalStatus::Passed {
                    proposal.execution_block = Some(current_block + self.config.execution_delay);
                }
            }
        }
    }

    /// Execute a proposal
    pub fn execute_proposal(&mut self, proposal_id: ProposalId, current_block: BlockHeight) -> Result<(), GovernanceError> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Passed {
            return Err(GovernanceError::ProposalNotPassed);
        }

        if let Some(exec_block) = proposal.execution_block {
            if current_block < exec_block {
                return Err(GovernanceError::ExecutionDelayNotPassed);
            }
        } else {
            return Err(GovernanceError::ExecutionBlockNotSet);
        }

        // Mark as executed
        proposal.status = ProposalStatus::Executed;
        Ok(())
    }

    /// Get proposal by ID
    pub fn get_proposal(&self, proposal_id: ProposalId) -> Option<&Proposal> {
        self.proposals.get(&proposal_id)
    }

    /// Get all active proposals
    pub fn get_active_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Active)
            .collect()
    }

    /// Get governance statistics
    pub fn stats(&self) -> GovernanceStats {
        let total = self.proposals.len();
        let active = self.proposals.values().filter(|p| p.status == ProposalStatus::Active).count();
        let passed = self.proposals.values().filter(|p| p.status == ProposalStatus::Passed).count();
        let executed = self.proposals.values().filter(|p| p.status == ProposalStatus::Executed).count();
        let rejected = self.proposals.values().filter(|p| p.status == ProposalStatus::Rejected).count();

        GovernanceStats {
            total_proposals: total,
            active_proposals: active,
            passed_proposals: passed,
            executed_proposals: executed,
            rejected_proposals: rejected,
            total_voters: self.voting_power.len(),
        }
    }
}

/// Governance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceStats {
    pub total_proposals: usize,
    pub active_proposals: usize,
    pub passed_proposals: usize,
    pub executed_proposals: usize,
    pub rejected_proposals: usize,
    pub total_voters: usize,
}

/// Governance errors
#[derive(Debug, thiserror::Error)]
pub enum GovernanceError {
    #[error("insufficient deposit")]
    InsufficientDeposit,
    
    #[error("no voting power")]
    NoVotingPower,
    
    #[error("proposal not found")]
    ProposalNotFound,
    
    #[error("voting has not started")]
    VotingNotStarted,
    
    #[error("voting has ended")]
    VotingEnded,
    
    #[error("proposal is not active")]
    ProposalNotActive,
    
    #[error("proposal has not passed")]
    ProposalNotPassed,
    
    #[error("execution delay has not passed")]
    ExecutionDelayNotPassed,
    
    #[error("execution block not set")]
    ExecutionBlockNotSet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenomics::TOTAL_SUPPLY;

    #[test]
    fn test_create_proposal() {
        let config = GovernanceConfig::default();
        // Use actual total supply from tokenomics
        let mut gov = GovernanceSystem::new(config.clone(), TOTAL_SUPPLY);
        
        let proposer = Address::from_bytes([1u8; 20]);
        // 5000 LAT voting power (5000 * 10^8)
        gov.update_voting_power(proposer.clone(), 500_000_000_000);
        
        let proposal_id = gov.create_proposal(
            proposer,
            ProposalType::TextProposal {
                title: "Test".to_string(),
                description: "Test proposal".to_string(),
            },
            "Test description".to_string(),
            100,
            config.min_deposit,
        ).unwrap();
        
        assert_eq!(proposal_id, 1);
    }

    #[test]
    fn test_voting() {
        let config = GovernanceConfig::default();
        // Use actual total supply from tokenomics
        let mut gov = GovernanceSystem::new(config.clone(), TOTAL_SUPPLY);
        
        let proposer = Address::from_bytes([1u8; 20]);
        let voter1 = Address::from_bytes([2u8; 20]);
        let voter2 = Address::from_bytes([3u8; 20]);
        
        // Voting power in Latt (8 decimals)
        gov.update_voting_power(proposer.clone(), 500_000_000_000);  // 5000 LAT
        gov.update_voting_power(voter1.clone(), 300_000_000_000);    // 3000 LAT
        gov.update_voting_power(voter2.clone(), 200_000_000_000);    // 2000 LAT
        
        let proposal_id = gov.create_proposal(
            proposer,
            ProposalType::TextProposal {
                title: "Test".to_string(),
                description: "Test proposal".to_string(),
            },
            "Test".to_string(),
            100,
            config.min_deposit,
        ).unwrap();
        
        // Vote
        gov.vote(proposal_id, voter1, VoteChoice::Yes, 200).unwrap();
        gov.vote(proposal_id, voter2, VoteChoice::No, 200).unwrap();
        
        let proposal = gov.get_proposal(proposal_id).unwrap();
        assert!(proposal.yes_votes > 0);
        assert!(proposal.no_votes > 0);
    }
}
