use serde::{Serialize, Deserialize};
use core::{fmt, fmt::Display};

#[allow(non_snake_case)]
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

impl Display for EthTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();
        output += "Transaction Details:\n";
        
        let block_number = match self.blockNumber {
            Some(ref bn) => format!("    blockNumber:       {}\n", bn),
            None => "".to_string()
        };

        let time_stamp = match self.timeStamp {
            Some(ref ts) => format!("    timeStamp:          {}\n", ts),
            None => "".to_string()
        };

        let hash = match self.hash {
            Some(ref ha) => format!("    hash:               {}\n", ha),
            None => "".to_string()
        };

        let nonce = match self.nonce {
            Some(ref no) => format!("    nonce:              {}\n", no),
            None => "".to_string()
        };

        let block_hash = match self.blockHash {
            Some(ref bh) => format!("    blockHash:          {}\n", bh),
            None => "".to_string()
        };

        let from = match self.from {
            Some(ref fr) => format!("    from:               {}\n", fr),
            None => "".to_string()
        };

        let contract_address = match self.contractAddress {
            Some(ref ca) => format!("    contractAddress:    {}\n", ca),
            None => "".to_string()
        };

        let to = match self.to {
            Some(ref to) => format!("    to:                 {}\n", to),
            None => "".to_string()
        };

        let value = match self.value {
            Some(ref va) => format!("    value:              {}\n", va),
            None => "".to_string()
        };

        let token_name = match self.tokenName {
            Some(ref tn) => format!("    tokenName:          {}\n", tn),
            None => "".to_string()
        };

        let token_symbol = match self.tokenSymbol {
            Some(ref ts) => format!("    tokenSymbol:        {}\n", ts),
            None => "".to_string()
        };

        let token_decimal = match self.tokenDecimal {
            Some(ref td) => format!("    tokenDecimal:       {}\n", td),
            None => "".to_string()
        };

        let transaction_index = match self.transactionIndex {
            Some(ref ti) => format!("    transactionIndex:   {}\n", ti),
            None => "".to_string()
        };

        let gas = match self.gas {
            Some(ref ga) => format!("    gas:                {}\n", ga),
            None => "".to_string()
        };

        let gas_price = match self.gasPrice {
            Some(ref gp) => format!("    gasPrice:           {}\n", gp),
            None => "".to_string()
        };

        let gas_used = match self.gasUsed {
            Some(ref gu) => format!("    gasUsed:            {}\n", gu),
            None => "".to_string()
        };

        let cumulative_gas_used = match self.cumulativeGasUsed {
            Some(ref cgu) => format!("    cumulativeGasUsed:  {}\n", cgu),
            None => "".to_string()
        };

        let input = match self.input {
            Some(ref inp) => format!("    input:              {}\n", inp),
            None => "".to_string()
        };

        let confirmations = match self.confirmations {
            Some(ref c) => format!("    confirmations:       {}", c),
            None => "".to_string()
        };

        output += &block_number;
        output += &time_stamp;
        output += &hash;
        output += &nonce;
        output += &block_hash;
        output += &from;
        output += &contract_address;
        output += &to;
        output += &value;
        output += &token_name;
        output += &token_symbol;
        output += &token_decimal;
        output += &transaction_index;
        output += &gas;
        output += &gas_price;
        output += &gas_used;
        output += &cumulative_gas_used;
        output += &input;
        output += &confirmations;

        write!(f, "\n{}", output)
    }
}
