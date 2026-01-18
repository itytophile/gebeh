#[derive(Clone, Default)]
struct EnvelopeTimer {
    value: u8, // 4 bits
    is_increasing: bool,
    sweep_pace: u8, // 3 bits
    pace_count: u8,
}

impl EnvelopeTimer {
    fn trigger(&mut self, volume_and_envelope: u8) {
        self.is_increasing = volume_and_envelope & 0x08 != 0;
        self.value = (volume_and_envelope >> 4) & 0x0f;
        self.sweep_pace = volume_and_envelope & 0x07;
        self.pace_count = 0;
    }
    fn tick(&mut self) {
        // https://gbdev.io/pandocs/Audio_Registers.html#ff12--nr12-channel-1-volume--envelope
        // A setting of 0 disables the envelope.
        // about multiple of 8 https://gbdev.io/pandocs/Audio_details.html#div-apu
        if self.sweep_pace == 0 {
            return;
        }

        self.pace_count += 1;

        if self.pace_count != self.sweep_pace {
            return;
        }

        self.pace_count = 0;

        match (self.is_increasing, self.value) {
            (true, 0x0f) | (false, 0) => {}
            (true, _) => self.value += 1,
            (false, _) => self.value -= 1,
        }
    }
}

#[derive(Clone, Default)]
pub struct VolumeAndEnvelope {
    timer: EnvelopeTimer,
    register: u8,
}

impl VolumeAndEnvelope {
    pub fn is_dac_on(&self) -> bool {
        // https://gbdev.io/pandocs/Audio_details.html#dacs
        self.register & 0xf8 != 0
    }

    pub fn get_volume(&self) -> u8 {
        self.timer.value
    }

    pub fn get_register(&self) -> u8 {
        self.register
    }

    pub fn write_register(&mut self, value: u8) {
        self.register = value;
    }

    pub fn trigger(&mut self) {
        self.timer.trigger(self.register);
    }

    pub fn tick(&mut self) {
        self.timer.tick();
    }
}
