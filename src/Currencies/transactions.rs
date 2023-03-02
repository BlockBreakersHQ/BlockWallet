use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EthTransaction {
    pub blockNumber         : Option<String>,
    pub timeStamp           : Option<String>,
    pub hash                : Option<String>,
    pub nonce               : Option<String>,
    pub blockHash           : Option<String>,
    pub from                : Option<String>,
    pub contractAddress     : Option<String>,
    pub to                  : Option<String>,
    pub value               : Option<String>,
    pub tokenName           : Option<String>,
    pub tokenSymbol         : Option<String>,
    pub tokenDecimal        : Option<String>,
    pub transactionIndex    : Option<String>,
    pub gas                 : Option<String>,
    pub gasPrice            : Option<String>,
    pub gasUsed             : Option<String>,
    pub cumulativeGasUsed   : Option<String>,
    pub input               : Option<String>,
    pub confirmations       : Option<String>
}