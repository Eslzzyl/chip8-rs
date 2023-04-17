use rand::random;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const START_ADDR: u16 = 0x200;

const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Emu {
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_regs: [u8; NUM_REGS],
    i_reg: u16,
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    delay_timer: u8,
    sound_timer: u8
}

impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_regs: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            delay_timer: 0,
            sound_timer: 0
        };

        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        new_emu
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn push(&mut self, x: u16) {
        self.stack[self.sp as usize] = x;
        self.sp += 1;
    }

    pub fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn tick(&mut self) {
        let opcode = self.fetch();
        self.execute(opcode);
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                //Beep
                
            }
            self.sound_timer -= 1;
        }
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn key_press(&mut self, key_index: usize, pressed: bool) {
        self.keys[key_index] = pressed;
    }

    pub fn load(&mut self, bytes: &[u8]) {
        let start = START_ADDR as usize;
        let end = START_ADDR as usize + bytes.len();
        self.ram[start..end].copy_from_slice(bytes);
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        self.pc += 2;
        (higher_byte << 8) + lower_byte
    }

    fn execute(&mut self, opcode: u16) {
        let digit1 = (opcode & 0xF000) >> 12;
        let digit2 = (opcode & 0x0F00) >> 8;
        let digit3 = (opcode & 0x00F0) >> 4;
        let digit4 = opcode & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            (0, 0, 0, 0) => return,                     //nop
            (0, 0, 0xE, 0) => self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT],    //cls
            (0, 0, 0xE, 0xE) => self.pc = self.pop(),   //ret
            (1, _, _, _) => self.pc = opcode & 0xFFF,   //jmp
            (2, _, _, _) => {                           //call
                self.push(self.pc);
                self.pc = opcode & 0x0FFF;
            },
            (3, _, _, _) => {       //3VNN: skip next inst if v[v] == NN
                if self.v_regs[digit2 as usize] as u16 == opcode & 0x00FF {
                    self.pc += 2;
                }
            },
            (4, _, _, _) => {       //4VNN: skip next inst if v != nn
                if self.v_regs[digit2 as usize] as u16 != opcode & 0x00FF {
                    self.pc += 2;
                }
            },
            (5, _, _, 0) => {       //5XY0: skip next inst if v[x] == v[y]
                if self.v_regs[digit2 as usize] == self.v_regs[digit3 as usize] {
                    self.pc += 2;
                }
            },
            (6, _, _, _) => {       //6XNN: v[x] = nn
                self.v_regs[digit2 as usize] = (opcode & 0x00FF) as u8;
            },
            (7, _, _, _) => {       //7XNN: v[x] += nn
                let x = digit2 as usize;
                self.v_regs[x] = self.v_regs[x].wrapping_add((opcode & 0x00FF) as u8);
            },
            (8, _, _, 0) => {       //8XY0: v[x] = v[y]
                self.v_regs[digit2 as usize] = self.v_regs[digit3 as usize];
            },
            (8, _, _, 1) => {       //8XY1: v[x] = v[x] | v[y]
                self.v_regs[digit2 as usize] |= self.v_regs[digit3 as usize];
            },
            (8, _, _, 2) => {       //8XY2: v[x] = v[x] & v[y]
                self.v_regs[digit2 as usize] &= self.v_regs[digit3 as usize];
            },
            (8, _, _, 3) => {       //8XY3: v[x] = v[x] ^ v[y]
                self.v_regs[digit2 as usize] ^= self.v_regs[digit3 as usize];
            },
            (8, _, _, 4) => {       //8XY4: v[x] += v[y], and if overflow occured then set VF(v[15]) to 1.
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (result, carry) = self.v_regs[x].overflowing_add(self.v_regs[y]);
                self.v_regs[0xF] = if carry { 1 } else { 0 };
                self.v_regs[x] = result;
            },
            (8, _, _, 5) => {       //8XY5: v[x] -= v[y], but the carry flag works in the opposite fashion.
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (result, borrow) = self.v_regs[x].overflowing_sub(self.v_regs[y]);
                self.v_regs[0xF] = if borrow { 0 } else { 1 };
                self.v_regs[x] = result;
            },
            (8, _, _, 6) => {       //8XY6
                let x = digit2 as usize;
                let lsb = self.v_regs[x] & 1;
                self.v_regs[x] >>= 1;
                self.v_regs[0xF] = lsb;
            },
            (8, _, _, 7) => {       //8XY7: v[x] = v[y] - v[x].
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (result, borrow) = self.v_regs[y].overflowing_sub(self.v_regs[x]);
                self.v_regs[0xF] = if borrow { 0 } else { 1 };
                self.v_regs[x] = result;
            },
            (8, _, _, 0xE) => {     //8XYE
                let x = digit2 as usize;
                let msb = (self.v_regs[x] >> 7) & 1;
                self.v_regs[x] <<= 1;
                self.v_regs[0xF] = msb;
            },
            (9, _, _, 0) => {       //9XY0: skip if v[x] != v[y]
                if self.v_regs[digit2 as usize] != self.v_regs[digit3 as usize] {
                    self.pc += 2;
                }
            },
            (0xA, _, _, _) => {     //ANNN: i = nnn
                self.i_reg = opcode & 0x0FFF;
            },
            (0xB, _, _, _) => {     //BNNN: jump to nnn+v[0]
                self.pc = (opcode & 0x0FFF) + self.v_regs[0] as u16;
            },
            (0xC, _, _, _) => {     //CXNN: v[x] = rand() & nn
                self.v_regs[digit2 as usize] = random::<u8>() & (opcode & 0x00FF) as u8;
            },
            (0xD, _, _, _) => {     //DXYN: Draw Sprite
                let x_coord = self.v_regs[digit2 as usize] as u16;
                let y_coord = self.v_regs[digit3 as usize] as u16;
                let num_rows = digit4;
                let mut flipped = false;
                for y_line in 0..num_rows {
                    let addr = self.i_reg + y_line as u16;
                    let pixels = self.ram[addr as usize];
                    for x_line in 0..8 {
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            // Sprites should wrap around screen, so apply modulo
                            let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                            let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;
                            // Get our pixel's index for our 1D screen array
                            let idx = x + SCREEN_WIDTH * y;
                            // Check if we're about to flip the pixel and set
                            flipped |= self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }
                // Populate VF register
                if flipped {
                    self.v_regs[0xF] = 1;
                } else {
                    self.v_regs[0xF] = 0;
                }
            },
            (0xE, _, 9, 0xE) => {       //EX9E: skip if key pressed
                if self.keys[self.v_regs[digit2 as usize] as usize] {
                    self.pc += 2;
                }
            },
            (0xE, _, 0xA, 1) => {       //EXA1: skip if key not pressed
                if !self.keys[self.v_regs[digit2 as usize] as usize] {
                    self.pc += 2;
                }
            },
            (0xF, _, 0, 7) => {         //FX07: v[x] = delay_timer
                self.v_regs[digit2 as usize] = self.delay_timer;
            },
            (0xF, _, 0, 0xA) => {       //FX0A: wait for key press
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_regs[digit2 as usize] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                if !pressed {
                    self.pc -= 2;
                }
            },
            (0xF, _, 1, 5) => {         //FX15: delay_timer = v[x]
                self.delay_timer = self.v_regs[digit2 as usize];
            },
            (0xF, _, 1, 8) => {         //FX18: sound_timer = v[x]
                self.sound_timer = self.v_regs[digit2 as usize];
            },
            (0xF, _, 1, 0xE) => {       //FX1E: i += v[x]
                self.i_reg = self.i_reg.wrapping_add(self.v_regs[digit2 as usize] as u16);
            },
            (0xF, _, 2, 9) => {         //FX29: set i to font address
                self.i_reg = 5 * self.v_regs[digit2 as usize] as u16;
            },
            (0xF, _, 3, 3) => {         //FX33: i = BCD of v[x]
                let vx = self.v_regs[digit2 as usize] as f32;
                // Fetch the hundreds digit by dividing by 100 and tossing the decimal
                let hundreds = (vx / 100.0).floor() as u8;
                // Fetch the tens digit by dividing by 10, tossing the ones digit and the decimal
                let tens = ((vx / 10.0) % 10.0).floor() as u8;
                // Fetch the ones digit by tossing the hundreds and the tens
                let ones = (vx % 10.0) as u8;
                self.ram[self.i_reg as usize] = hundreds;
                self.ram[(self.i_reg + 1) as usize] = tens;
                self.ram[(self.i_reg + 2) as usize] = ones;
            },
            (0xF, _, 5, 5) => {         //FX55: store v[0] to v[x] into i
                for idx in 0..=(digit2 as usize) {
                    self.ram[self.i_reg as usize] = self.v_regs[idx];
                }
            },
            (0xF, _, 6, 5) => {         //FX65: load i into v[0] to v[x]
                for idx in 0..=(digit2 as usize) {
                    self.v_regs[idx] = self.ram[self.i_reg as usize];
                }
            },
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", opcode),
        }
    }

}