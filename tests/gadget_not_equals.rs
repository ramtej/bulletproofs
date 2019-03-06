extern crate bulletproofs;
extern crate curve25519_dalek;
extern crate merlin;

use bulletproofs::r1cs::{ConstraintSystem, R1CSError, R1CSProof, Variable, Prover, Verifier};
use curve25519_dalek::scalar::Scalar;
use bulletproofs::{BulletproofGens, PedersenGens};
use curve25519_dalek::ristretto::CompressedRistretto;
use bulletproofs::r1cs::LinearCombination;

mod utils;
use utils::{AllocatedScalar, is_nonzero_gadget};

// Ensure `v` is not equal to expected
pub fn not_equals_gadget<CS: ConstraintSystem>(
    cs: &mut CS,
    v: AllocatedScalar,
    diff_var: AllocatedScalar,
    diff_inv_var: AllocatedScalar,
    expected: &u64
) -> Result<(), R1CSError> {
    let expected_lc: LinearCombination = vec![(Variable::One(), Scalar::from(*expected))].iter().collect();
    let v_minus_expected_lc = v.variable - expected_lc;

    // Since `diff_var` is `expected - v`, `v - expected` + `diff_var` should be 0
    cs.constrain(diff_var.variable + v_minus_expected_lc);

    // Ensure `set[i] - v` is non-zero
    is_nonzero_gadget(cs, diff_var, diff_inv_var)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use merlin::Transcript;

    #[test]
    fn test_not_equals_gadget() {
        // Check that committed value is not equal to a public value
        let value = 10u64;
        let expected = 5u64;

        assert!(not_equals_gadget_helper(value, expected).is_ok());
    }

    // Prove that difference between value and expected is non-zero, hence val does not equal the expected.
    fn not_equals_gadget_helper(val: u64, expected: u64) -> Result<(), R1CSError> {
        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(128, 1);

        let (proof, commitments) = {
            let mut comms: Vec<CompressedRistretto> = vec![];

            let mut prover_transcript = Transcript::new(b"NotEqualsTest");
            let mut rng = rand::thread_rng();
            let mut prover = Prover::new(&bp_gens, &pc_gens, &mut prover_transcript);

            let value= Scalar::from(val);
            let (com_value, var_value) = prover.commit(value.clone(), Scalar::random(&mut rng));
            let alloc_scal = AllocatedScalar {
                variable: var_value,
                assignment: Some(value),
            };
            comms.push(com_value);

            let diff = Scalar::from(expected) - value;
            let (com_diff, var_diff) = prover.commit(diff.clone(), Scalar::random(&mut rng));
            let alloc_scal_diff = AllocatedScalar {
                variable: var_diff,
                assignment: Some(diff),
            };
            comms.push(com_diff);

            let diff_inv = diff.invert();
            let (com_diff_inv, var_diff_inv) = prover.commit(diff_inv.clone(), Scalar::random(&mut rng));
            let alloc_scal_diff_inv = AllocatedScalar {
                variable: var_diff_inv,
                assignment: Some(diff_inv),
            };
            comms.push(com_diff_inv);

            assert!(not_equals_gadget(&mut prover, alloc_scal, alloc_scal_diff, alloc_scal_diff_inv, &expected).is_ok());

            let proof = prover.prove()?;

            (proof, comms)
        };

        let mut verifier_transcript = Transcript::new(b"NotEqualsTest");
        let mut verifier = Verifier::new(&bp_gens, &pc_gens, &mut verifier_transcript);

        let var_val = verifier.commit(commitments[0]);
        let alloc_scal = AllocatedScalar {
            variable: var_val,
            assignment: None,
        };

        let var_diff = verifier.commit(commitments[1]);
        let alloc_scal_diff = AllocatedScalar {
            variable: var_diff,
            assignment: None,
        };

        let var_diff_inv = verifier.commit(commitments[2]);
        let alloc_scal_diff_inv = AllocatedScalar {
            variable: var_diff_inv,
            assignment: None,
        };

        assert!(not_equals_gadget(&mut verifier, alloc_scal, alloc_scal_diff, alloc_scal_diff_inv, &expected).is_ok());

        Ok(verifier.verify(&proof)?)
    }
}