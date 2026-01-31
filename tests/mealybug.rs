use std::{fs::File, io::BufReader};

#[test]
fn prout() {
    let decoder = png::Decoder::new(BufReader::new(
        File::open(
            "downloads/mealybug-tearoom-tests-master/expected/DMG-blob/m2_win_en_toggle.png",
        )
        .unwrap(),
    ));
    let mut reader = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    // Grab the bytes of the image.
    let bytes = &buf[..info.buffer_size()];
    println!("{info:?} {} {}", bytes.len(), buf.len())
}
