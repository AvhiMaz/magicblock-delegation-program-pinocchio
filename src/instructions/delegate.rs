use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{Sysvar, rent::Rent},
};

use crate::{
    states::{
        DELEGATION_PROGRAM_ID, close_pda_acc, cpi_delegate, deserialize_delegate_ix_data, get_seeds,
    },
    types::DelegateAccountArgs,
};

pub const BUFFER: &[u8] = b"buffer";

pub fn _process_delegation(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // payer: Signer who pays for account creations/rents. Usually the client.
    // pda_acc: Program Derived Address (PDA) owned by your program. Stores counter state or delegation info.
    // owner_program: The program that originally owns or controls the state being delegated (e.g. the counter program).
    // buffer_acc: Temporary buffer account (could store serialized state during delegation or rollback).
    // delegation_record: A new or existing account that records the delegation event (who delegated, what, when).
    // delegation_metadata: Possibly stores metadata like TTL, rollup ID, hash commitments, or state proof refs.
    // system_program: Required when creating new accounts via CPI. Standard Solana system program.
    let [
        payer,
        pda_acc,
        owner_program,
        buffer_acc,
        delegation_record,
        delegation_metadata,
        system_program,
        _rest @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // checking if payer is signer or not
    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // It’s deserializing the instruction data that was passed along with the transaction.
    // seeds:These are the values used to generate the Program-Derived Address (PDA) for the account (pda_acc) that stores the state being delegated.
    // config: This would be the configuration or metadata of the delegation.
    let (seeds, config) = deserialize_delegate_ix_data(instruction_data)?;

    // Cloning is needed here because you may need to use seeds_data later in different places,
    // is it avoidable ?
    // This is essentially the primary seed used to generate the Program Derived Address (PDA) that represents the state (like a Counter, Vault, etc.) that you're trying to delegate or operate on.
    let delegate_pda_seeds = seeds.clone();

    // buffer_seeds is used to generate a different PDA — one that likely stores temporary data or is used as an intermediary in your program's logic.
    let buffer_seeds: &[&[u8]] = &[BUFFER, pda_acc.key().as_ref()];
    // seeds_data.iter() iterates over each item in seeds_data.
    // map(|s| s.as_slice()) turns each element into a slice (&[u8]).
    // .collect() collects the results into a Vec<&[u8]>, which will be used to generate the PDA.
    let pda_seeds: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();

    let (_, delegate_account_bump) = pubkey::find_program_address(&pda_seeds, &crate::ID);
    let (_, buffer_pda_bump) = pubkey::find_program_address(buffer_seeds, &crate::ID);

    //DELEGATE PDA SIGNER SEEDS
    //************************************************************************************************************************************************************
    // Just wrapping the bump in a slice (&[u8]).
    let binding = &[delegate_account_bump];
    // Converting the bump slice into a format usable by your PDA signer setup.
    let delegate_bump = Seed::from(binding);
    // get_seeds() likely turns your pda_seeds (like vec![b"counter", payer.key().as_ref()]) into a vector of Seed types (e.g., Vec<&[u8]>).
    let mut delegate_seeds = get_seeds(pda_seeds)?;
    // Append the bump to the seed list.
    delegate_seeds.extend_from_slice(&[delegate_bump]);
    // Finally, package the complete seeds into a structure (Signer) that invoke_signed() can use.
    let delegate_signer_seeds = Signer::from(delegate_seeds.as_slice());
    //
    //************************************************************************************************************************************************************
    // can we do it like this ?
    //
    //let delegate_bump = &[delegate_account_bump];
    //let seed_d = [
    //    Seed::from(b"delegate"),
    //    Seed::from(pda_acc.key().as_ref()),
    //    Seed::from(delegate_bump),
    //];
    //
    //let delegate_signer_seeds = Signer::from(&seed_d)

    // BUFFER PDA SIGNER SEEDS
    //************************************************************************************************************************************************************
    let buffer_bump = [buffer_pda_bump];
    let seed_b = [
        Seed::from(b"buffer"),
        Seed::from(pda_acc.key().as_ref()),
        Seed::from(&buffer_bump),
    ];
    let buffer_signer_seeds = Signer::from(&seed_b);
    //
    //************************************************************************************************************************************************************

    //create buffer pda account
    //
    pinocchio_system::instructions::CreateAccount {
        from: payer,
        to: buffer_acc,
        lamports: Rent::get()?.minimum_balance(pda_acc.data_len()),
        space: pda_acc.data_len() as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[buffer_signer_seeds.clone()])?;

    let mut buffer_data = buffer_acc.try_borrow_mut_data()?;
    let new_data = pda_acc.try_borrow_data()?.to_vec().clone();
    (*buffer_data).copy_from_slice(&new_data);
    drop(buffer_data);

    //Close Delegate PDA in preparation for CPI Delegate
    close_pda_acc(payer, pda_acc, system_program)?;

    //we create account with Delegation Account
    pinocchio_system::instructions::CreateAccount {
        from: payer,
        to: pda_acc,
        lamports: Rent::get()?.minimum_balance(pda_acc.data_len()),
        space: pda_acc.data_len() as u64, //PDA acc length
        owner: &DELEGATION_PROGRAM_ID,
    }
    .invoke_signed(&[delegate_signer_seeds.clone()])?;

    //preprare delegate args
    //struct DelegateConfig comes from IX data
    let delegate_args = DelegateAccountArgs {
        commit_frequency_ms: config.commit_frequency_ms,
        seeds: delegate_pda_seeds,
        validator: config.validator,
    };

    cpi_delegate(
        payer,
        pda_acc,
        owner_program,
        buffer_acc,
        delegation_record,
        delegation_metadata,
        system_program,
        delegate_args,
        delegate_signer_seeds,
    )?;

    close_pda_acc(payer, buffer_acc, system_program)?;

    Ok(())
}

