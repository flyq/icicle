use icicle_runtime::{
    memory::{DeviceVec, HostSlice},
    stream::IcicleStream,
};

// Using both bn254 and bls12-377 curves
use icicle_bls12_377::curve::{
    CurveCfg as BLS12377CurveCfg, G1Projective as BLS12377G1Projective, ScalarCfg as BLS12377ScalarCfg,
};
use icicle_bn254::curve::{CurveCfg, G1Projective, G2CurveCfg, G2Projective, ScalarCfg};

use clap::Parser;
use icicle_core::{curve::Curve, msm, traits::GenerateRandom};
use std::time::Instant;

#[derive(Parser, Debug)]
struct Args {
    /// Lower bound (inclusive) of MSM sizes to run for
    #[arg(short, long, default_value_t = 20)]
    lower_bound_log_size: u8,

    /// Upper bound of MSM sizes to run for
    #[arg(short, long, default_value_t = 20)]
    upper_bound_log_size: u8,

    /// Device type (e.g., "CPU", "CUDA")
    #[arg(short, long, default_value = "CUDA")]
    device_type: String,
}

// Load backend and set device
fn try_load_and_set_backend_device(args: &Args) {
    if args.device_type != "CPU" {
        icicle_runtime::runtime::load_backend_from_env_or_default().unwrap();
    }
    println!("Setting device {}", args.device_type);
    let device = icicle_runtime::Device::new(&args.device_type, 0 /* =device_id*/);
    icicle_runtime::set_device(&device).unwrap();
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    try_load_and_set_backend_device(&args);

    let lower_bound = args.lower_bound_log_size;
    let upper_bound = args.upper_bound_log_size;
    println!("Running Icicle Examples: Rust MSM");
    let upper_size = 1 << upper_bound;

    let generate_bn254_start = Instant::now();

    println!("Generating random inputs on host for bn254...");
    let upper_points = CurveCfg::generate_random_affine_points(upper_size);
    let g2_upper_points = G2CurveCfg::generate_random_affine_points(upper_size);
    let upper_scalars = ScalarCfg::generate_random(upper_size);

    let generate_bn254_duration = generate_bn254_start.elapsed();
    println!("generate bn254 points time: {:?}", generate_bn254_duration);

    let generate_bls12377_start = Instant::now();

    println!("Generating random inputs on host for bls12377...");
    let upper_points_bls12377 = BLS12377CurveCfg::generate_random_affine_points(upper_size);
    let upper_scalars_bls12377 = BLS12377ScalarCfg::generate_random(upper_size);

    let generate_bls12377_duration = generate_bls12377_start.elapsed();
    println!("generate bls12377 points time: {:?}", generate_bls12377_duration);

    for i in lower_bound..=upper_bound {
        let log_size = i;
        let size = 1 << log_size;
        println!(
            "---------------------- MSM size 2^{} = {} ------------------------",
            log_size, size
        );

            let host_load_bn254_start = Instant::now();

        // Setting Bn254 points and scalars
        let points = HostSlice::from_slice(&upper_points[..size]);
        let g2_points = HostSlice::from_slice(&g2_upper_points[..size]);
        let scalars = HostSlice::from_slice(&upper_scalars[..size]);

        let host_load_bn254_duration = host_load_bn254_start.elapsed();
        println!("host load bn254 time: {:?}", host_load_bn254_duration);

        let host_load_bls12377_start = Instant::now();

        // Setting bls12377 points and scalars
        let points_bls12377 = HostSlice::from_slice(&upper_points_bls12377[..size]);
        let scalars_bls12377 = HostSlice::from_slice(&upper_scalars_bls12377[..size]);

        let host_load_bls12377_duration = host_load_bls12377_start.elapsed();
        println!("host load bls12377 time: {:?}", host_load_bls12377_duration);

        println!("Configuring bn254 MSM...");
        let device_configure_bn254_start = Instant::now();

        let mut msm_results = DeviceVec::<G1Projective>::device_malloc(1).unwrap();
        let mut g2_msm_results = DeviceVec::<G2Projective>::device_malloc(1).unwrap();
        let mut stream = IcicleStream::create().unwrap();
        let mut g2_stream = IcicleStream::create().unwrap();
        let mut cfg = msm::MSMConfig::default();
        let mut g2_cfg = msm::MSMConfig::default();
        cfg.stream_handle = *stream;
        cfg.is_async = true;
        g2_cfg.stream_handle = *g2_stream;
        g2_cfg.is_async = true;

        let device_configure_bn254_duration = device_configure_bn254_start.elapsed();
        println!("device configure bn254 time: {:?}", device_configure_bn254_duration);


        println!("Configuring bls12377 MSM...");

        let device_configure_bls12377_start = Instant::now();

        let mut msm_results_bls12377 = DeviceVec::<BLS12377G1Projective>::device_malloc(1).unwrap();
        let mut stream_bls12377 = IcicleStream::create().unwrap();
        let mut cfg_bls12377 = msm::MSMConfig::default();
        cfg_bls12377.stream_handle = *stream_bls12377;
        cfg_bls12377.is_async = true;

        let device_configure_bls12377_duration = device_configure_bls12377_start.elapsed();
        println!("device configure bls12377 time: {:?}", device_configure_bls12377_duration);

        println!("Executing bn254 MSM on device...");
        let bn254_start_g1 = Instant::now();

        msm::msm(scalars, points, &cfg, &mut msm_results[..]).unwrap();

        let bn254_duration_g1 = bn254_start_g1.elapsed();
        println!("bn254 msm time: {:?}", bn254_duration_g1);

        let bn254_start_g2 = Instant::now();
        msm::msm(scalars, g2_points, &g2_cfg, &mut g2_msm_results[..]).unwrap();

        let bn254_duration_g2 = bn254_start_g2.elapsed();
        println!("bn254 msm time: {:?}", bn254_duration_g2);

        println!("Executing bls12377 MSM on device...");
        let bls12377_start = Instant::now();

        msm::msm(
            scalars_bls12377,
            points_bls12377,
            &cfg_bls12377,
            &mut msm_results_bls12377[..],
        )
        .unwrap();

        let bls12377_duration = bls12377_start.elapsed();
        println!("bls12377 msm time: {:?}", bls12377_duration);

        println!("Moving results to host...");
        let move_bn254_to_host_start = Instant::now();

        let mut msm_host_result = vec![G1Projective::zero(); 1];
        let mut g2_msm_host_result = vec![G2Projective::zero(); 1];

        stream
            .synchronize()
            .unwrap();
        msm_results
            .copy_to_host(HostSlice::from_mut_slice(&mut msm_host_result[..]))
            .unwrap();
        println!("bn254 result: {:#?}", msm_host_result);

        g2_stream
            .synchronize()
            .unwrap();
        g2_msm_results
            .copy_to_host(HostSlice::from_mut_slice(&mut g2_msm_host_result[..]))
            .unwrap();
        println!("G2 bn254 result: {:#?}", g2_msm_host_result);

        let move_bn254_to_host_duration = move_bn254_to_host_start.elapsed();
        println!("move bn254 to host time: {:?}", move_bn254_to_host_duration);

        let move_bls12377_to_host_start = Instant::now();
        let mut msm_host_result_bls12377 = vec![BLS12377G1Projective::zero(); 1];

        stream_bls12377
            .synchronize()
            .unwrap();
        msm_results_bls12377
            .copy_to_host(HostSlice::from_mut_slice(&mut msm_host_result_bls12377[..]))
            .unwrap();
        println!("bls12377 result: {:#?}", msm_host_result_bls12377);

        let move_bls12377_to_host_duration = move_bls12377_to_host_start.elapsed();
        println!("move bls12377 to host time: {:?}", move_bls12377_to_host_duration);

        println!("Cleaning up bn254...");
        stream
            .destroy()
            .unwrap();
        g2_stream
            .destroy()
            .unwrap();

        println!("Cleaning up bls12377...");
        stream_bls12377
            .destroy()
            .unwrap();
        println!("");
    }
}
