//Copyright (c) 2020 Marco Boneberger
/*!
*  s_curve
*  ===
*  A library to create S-Curve with constrained jerk, acceleration and velocity.
*  It can be used to create a function which desribes the  Position, Velocity, Acceleration or Jerk
*  with respect to time. You can create a Motion Profile for robots with it.
* [![image](http://i.imgur.com/BQPhS8n.png)](http://i.imgur.com/BQPhS8n.png)
*
* The notation follows loosely the book
* "Trajectory Planning for Automatic Machinesand Robots" by Luigi Biagotti and Claudio Melchiorri
*  # Example
*  ```rust
*  use s_curve::*;
*  let constraints = SCurveConstraints {
*              max_jerk: 3.,
*              max_acceleration: 2.0,
*              max_velocity: 3.};
*          let  start_conditions = SCurveStartConditions {
*              q0: 0., // start position
*              q1: 10., // end position
*              v0: 0., // start velocity
*              v1: 0. // end velocity
*          };
*          let input = SCurveInput{constraints, start_conditions};
*          let (params,s_curve) = s_curve_generator(&input,Derivative::Velocity);
*          for i in 0..101 {
*              println!("{}", s_curve(i as f64 * params.time_intervals.total_duration() / 100.));
*          }
*  ```
*
*/

/**
 * Struct which contains the desired limits for jerk, acceleration and velocity in SI units.
 *  These are only the limits. It can happen that the acceleration or Velocity will be actually lower
 *  after the S-Curve has been calculated. The actual maximum values are in the SCurveParameters struct
 */
#[derive(Clone, Debug, Default)]
pub struct SCurveConstraints {
    pub max_jerk: f64,
    pub max_acceleration: f64,
    pub max_velocity: f64,
}

/// Enum which is used to select whether you want to calculate
/// Position, Velocity, Acceleration or Jerk of your S-Curve
pub enum Derivative {
    Position = 0,
    Velocity = 1,
    Acceleration = 2,
    Jerk = 3,
}

/// Represents the different time intervals of the S-Curve
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SCurveTimeIntervals {
    ///  time-interval in which the jerk is constant (j max or j min ) during the acceleration phase
    pub t_j1: f64,
    /// time-interval in which the jerk is constant (j max or j min ) during the deceleration phase
    pub t_j2: f64,
    ///   Acceleration period
    pub t_a: f64,
    ///  constant velocity period
    pub t_v: f64,
    ///   deceleration period
    pub t_d: f64,
}

impl SCurveTimeIntervals {
    /// calculates the total duration of the S-Curve
    pub fn total_duration(&self) -> f64 {
        self.t_a + self.t_d + self.t_v
    }
    fn is_max_acceleration_not_reached(&self) -> bool {
        self.t_a < 2. * self.t_j1 || self.t_d < 2. * self.t_j2
    }
}

/// Represents the Start and End Positions of the S_Curve
#[derive(Clone, Debug, PartialEq)]
pub struct SCurveStartConditions {
    /// start position
    pub q0: f64,
    /// end position
    pub q1: f64,
    ///start velocity
    pub v0: f64,
    ///end velocity
    pub v1: f64,
}

impl Default for SCurveStartConditions {
    fn default() -> Self {
        SCurveStartConditions {
            q0: 0.,
            q1: 1.,
            v0: 0.,
            v1: 0.,
        }
    }
}

impl SCurveStartConditions {
    /// displacement
    fn h(&self) -> f64 {
        f64::abs(self.q1 - self.q0)
    }
    /// direction
    fn dir(&self) -> f64 {
        if self.q1 < self.q0 {
            -1.0
        } else {
            1.0
        }
    }
}

/// Struct which represents the final parametrization of the S-Curve
#[derive(Clone, Debug, PartialEq)]
pub struct SCurveParameters {
    /// tine intervals of the Trajectory
    pub time_intervals: SCurveTimeIntervals,
    /// maximum jerk
    pub j_max: f64,
    /// minimum jerk
    pub j_min: f64,
    ///maximum achieved acceleration during the acceleration phase
    pub a_lim_a: f64,
    /// minimum achieved acceleration during the deceleration phase
    pub a_lim_d: f64,
    /// maximum  achieved velocity
    pub v_lim: f64,
    /// The start conditions of the S-Curve
    pub conditions: SCurveStartConditions,
}

impl SCurveParameters {
    pub fn new(times: &SCurveTimeIntervals, p: &SCurveInput) -> SCurveParameters {
        let dir = p.start_conditions.dir();
        let a_lim_a = p.constraints.max_jerk * times.t_j1;
        let a_lim_d = -p.constraints.max_jerk * times.t_j2;
        let v_lim = dir * (p.start_conditions.v0 + dir * (times.t_a - times.t_j1) * a_lim_a);
        SCurveParameters {
            time_intervals: times.clone(),
            j_max: p.constraints.max_jerk,
            j_min: -p.constraints.max_jerk,
            a_lim_a,
            a_lim_d,
            v_lim,
            conditions: p.start_conditions.clone(),
        }
    }
}

/// Struct which represent the input which is needed so that the S-Curve can be calculated
#[derive(Clone, Debug, Default)]
pub struct SCurveInput {
    pub constraints: SCurveConstraints,
    pub start_conditions: SCurveStartConditions,
}

impl SCurveInput {
    /// calculates the time intervals of the S-Curves
    pub fn calc_intervals(&self) -> SCurveTimeIntervals {
        self.calc_times_case_1()
    }
    /// checks if it is actually possible to accomplish a certain trajectory. Dont trust this function
    /// too much. But if it returns yes it is certainly doable. If it returns false it can still work by reducing acceleration and velocity
    pub fn is_trajectory_feasible(&self) -> bool {
        let t_j_star: f64 = f64::min(
            f64::sqrt(
                f64::abs(self.start_conditions.v1 - self.start_conditions.v0)
                    / self.constraints.max_jerk,
            ),
            self.constraints.max_acceleration / self.constraints.max_jerk,
        );
        if (t_j_star - self.constraints.max_acceleration / self.constraints.max_jerk).abs() < 1e-10
        {
            return self.start_conditions.h()
                > 0.5
                    * (self.start_conditions.v1 + self.start_conditions.v0)
                    * (t_j_star
                        + f64::abs(self.start_conditions.v1 - self.start_conditions.v0)
                            / self.constraints.max_acceleration);
        }
        if t_j_star < self.constraints.max_acceleration / self.constraints.max_jerk {
            return self.start_conditions.h()
                > t_j_star * (self.start_conditions.v0 + self.start_conditions.v1);
        }
        false
    }
    fn is_a_max_not_reached(&self) -> bool {
        let dir = self.start_conditions.dir();
        (self.constraints.max_velocity - dir * self.start_conditions.v0) * self.constraints.max_jerk
            < self.constraints.max_acceleration.powi(2)
    }
    fn is_a_min_not_reached(&self) -> bool {
        let dir = self.start_conditions.dir();
        (self.constraints.max_velocity - dir * self.start_conditions.v1) * self.constraints.max_jerk
            < self.constraints.max_acceleration.powi(2)
    }

    fn calc_times_case_1(&self) -> SCurveTimeIntervals {
        let mut times = SCurveTimeIntervals::default();
        let mut new_input = self.clone();
        let dir = self.start_conditions.dir();
        if self.is_a_max_not_reached() {
            times.t_j1 = f64::sqrt(
                (new_input.constraints.max_velocity - dir * self.start_conditions.v0)
                    / new_input.constraints.max_jerk,
            );
            times.t_a = 2. * times.t_j1;
        } else {
            times.t_j1 = new_input.constraints.max_acceleration / new_input.constraints.max_jerk;
            times.t_a = times.t_j1
                + (new_input.constraints.max_velocity - dir * self.start_conditions.v0)
                    / new_input.constraints.max_acceleration;
        }

        if self.is_a_min_not_reached() {
            times.t_j2 = f64::sqrt(
                (new_input.constraints.max_velocity - dir * self.start_conditions.v1)
                    / new_input.constraints.max_jerk,
            );
            times.t_d = 2. * times.t_j2;
        } else {
            times.t_j2 = new_input.constraints.max_acceleration / new_input.constraints.max_jerk;
            times.t_d = times.t_j2
                + (new_input.constraints.max_velocity - dir * self.start_conditions.v1)
                    / new_input.constraints.max_acceleration;
        }

        times.t_v = self.start_conditions.h() / new_input.constraints.max_velocity
            - times.t_a / 2.
                * (1. + dir * self.start_conditions.v0 / new_input.constraints.max_velocity)
            - times.t_d / 2.
                * (1. + dir * self.start_conditions.v1 / new_input.constraints.max_velocity);
        if times.t_v <= 0. {
            return self.calc_times_case_2(0);
        }
        if times.is_max_acceleration_not_reached() {
            new_input.constraints.max_acceleration *= 0.5;
            if new_input.constraints.max_acceleration > 0.01 {
                return new_input.calc_times_case_2(0);
            }
            new_input.constraints.max_acceleration = 0.;
        }
        self.handle_negative_acceleration_time(&mut times, &new_input);

        times
    }
    fn calc_times_case_2(&self, mut recursion_depth: i32) -> SCurveTimeIntervals {
        recursion_depth += 1;
        let mut times = self.get_times_case_2();
        let mut new_input = self.clone();
        if times.is_max_acceleration_not_reached() {
            new_input.constraints.max_acceleration *= 0.5;
            if new_input.constraints.max_acceleration > 0.01 {
                return new_input.calc_times_case_2(recursion_depth);
            }
            new_input.constraints.max_acceleration = 0.;
        }
        self.handle_negative_acceleration_time(&mut times, &new_input);
        if recursion_depth != 1 {
            new_input.constraints.max_acceleration *= 2.;
        }
        new_input.calc_times_case_2_precise(recursion_depth)
    }

    fn get_times_case_2(&self) -> SCurveTimeIntervals {
        let dir = self.start_conditions.dir();
        let t_j1 = self.constraints.max_acceleration / self.constraints.max_jerk;
        let t_j2 = self.constraints.max_acceleration / self.constraints.max_jerk;
        let delta = self.constraints.max_acceleration.powi(4) / self.constraints.max_jerk.powi(2)
            + 2. * (self.start_conditions.v0.powi(2) + self.start_conditions.v1.powi(2))
            + self.constraints.max_acceleration
                * (4. * self.start_conditions.h()
                    - 2. * self.constraints.max_acceleration / self.constraints.max_jerk
                        * (dir * self.start_conditions.v0 + dir * self.start_conditions.v1));
        let t_a = (self.constraints.max_acceleration.powi(2) / self.constraints.max_jerk
            - 2. * dir * self.start_conditions.v0
            + f64::sqrt(delta))
            / (2. * self.constraints.max_acceleration);
        let t_d = (self.constraints.max_acceleration.powi(2) / self.constraints.max_jerk
            - 2. * dir * self.start_conditions.v1
            + f64::sqrt(delta))
            / (2. * self.constraints.max_acceleration);
        let t_v = 0.;
        SCurveTimeIntervals {
            t_j1,
            t_j2,
            t_a,
            t_v,
            t_d,
        }
    }

    fn calc_times_case_2_precise(&self, mut _recursion_depth: i32) -> SCurveTimeIntervals {
        _recursion_depth += 1;
        let mut times = self.get_times_case_2();
        let mut new_input = self.clone();
        if times.is_max_acceleration_not_reached() {
            new_input.constraints.max_acceleration *= 0.99;
            if new_input.constraints.max_acceleration > 0.01 {
                return new_input.calc_times_case_2_precise(_recursion_depth);
            }
            new_input.constraints.max_acceleration = 0.;
        }
        self.handle_negative_acceleration_time(&mut times, &new_input);
        times
    }
    fn handle_negative_acceleration_time(
        &self,
        times: &mut SCurveTimeIntervals,
        new_input: &SCurveInput,
    ) {
        if times.t_a < 0. {
            times.t_j1 = 0.;
            times.t_a = 0.;
            times.t_d = 2. * self.start_conditions.h()
                / (self.start_conditions.v0 + self.start_conditions.v1);
            times.t_j2 = (new_input.constraints.max_jerk * self.start_conditions.h()
                - f64::sqrt(
                    new_input.constraints.max_jerk
                        * (new_input.constraints.max_jerk * self.start_conditions.h().powi(2)
                            + (self.start_conditions.v0 + self.start_conditions.v1).powi(2)
                                * (self.start_conditions.v1 - self.start_conditions.v0)),
                ))
                / (new_input.constraints.max_jerk
                    * (self.start_conditions.v1 + self.start_conditions.v0));
        }
        if times.t_d < 0. {
            times.t_j2 = 0.;
            times.t_d = 0.;
            times.t_a = 2. * self.start_conditions.h()
                / (self.start_conditions.v0 + self.start_conditions.v1);
            times.t_j2 = (new_input.constraints.max_jerk * self.start_conditions.h()
                - f64::sqrt(
                    new_input.constraints.max_jerk
                        * (new_input.constraints.max_jerk * self.start_conditions.h().powi(2)
                            - (self.start_conditions.v0 + self.start_conditions.v1).powi(2)
                                * (self.start_conditions.v1 - self.start_conditions.v0)),
                ))
                / (new_input.constraints.max_jerk
                    * (self.start_conditions.v1 + self.start_conditions.v0));
        }
    }
}

pub fn eval_position(p: &SCurveParameters, t: f64) -> f64 {
    let times = &p.time_intervals;
    if t < 0. {
        return p.conditions.q0;
    }
    let dir = p.conditions.dir();
    if t <= times.t_j1 {
        p.conditions.q0 + p.conditions.v0 * t + dir * p.j_max * t.powi(3) / 6.
    } else if t <= times.t_a - times.t_j1 {
        p.conditions.q0
            + p.conditions.v0 * t
            + dir * p.a_lim_a / 6. * (3. * t.powi(2) - 3. * times.t_j1 * t + times.t_j1.powi(2))
    } else if t <= times.t_a {
        p.conditions.q0 + dir * (p.v_lim + dir * p.conditions.v0) * times.t_a / 2.
            - dir * p.v_lim * (times.t_a - t)
            - dir * p.j_min * (times.t_a - t).powi(3) / 6.
    } else if t <= times.t_a + times.t_v {
        p.conditions.q0
            + dir * (p.v_lim + dir * p.conditions.v0) * times.t_a / 2.
            + dir * p.v_lim * (t - times.t_a)
    } else if t <= times.total_duration() - times.t_d + times.t_j2 {
        p.conditions.q1 - dir * (p.v_lim + dir * p.conditions.v1) * times.t_d / 2.
            + dir * p.v_lim * (t - times.total_duration() + times.t_d)
            - dir * p.j_max * (t - times.total_duration() + times.t_d).powi(3) / 6.
    } else if t <= times.total_duration() - times.t_j2 {
        p.conditions.q1 - dir * (p.v_lim + dir * p.conditions.v1) * times.t_d / 2.
            + dir * p.v_lim * (t - times.total_duration() + times.t_d)
            + dir * p.a_lim_d / 6.
                * (3. * (t - times.total_duration() + times.t_d).powi(2)
                    - 3. * times.t_j2 * (t - times.total_duration() + times.t_d)
                    + times.t_j2.powi(2))
    } else if t <= times.total_duration() {
        p.conditions.q1
            - p.conditions.v1 * (times.total_duration() - t)
            - dir * p.j_max * (times.total_duration() - t).powi(3) / 6.
    } else {
        p.conditions.q1
    }
}

pub fn eval_velocity(p: &SCurveParameters, t: f64) -> f64 {
    let times = &p.time_intervals;
    if t < 0. {
        return p.conditions.v0;
    }
    let dir = p.conditions.dir();
    if t <= times.t_j1 {
        p.conditions.v0 + dir * p.j_max * t.powi(2) / 2.
    } else if t <= times.t_a - times.t_j1 {
        p.conditions.v0 + dir * p.a_lim_a * (t - times.t_j1 / 2.)
    } else if t <= times.t_a {
        dir * p.v_lim + dir * p.j_min * (times.t_a - t).powi(2) / 2.
    } else if t <= times.t_a + times.t_v {
        dir * p.v_lim
    } else if t <= times.total_duration() - times.t_d + times.t_j2 {
        dir * p.v_lim - dir * p.j_max * (t - times.total_duration() + times.t_d).powi(2) / 2.
    } else if t <= times.total_duration() - times.t_j2 {
        dir * p.v_lim + dir * p.a_lim_d * (t - times.total_duration() + times.t_d - times.t_j2 / 2.)
    } else if t <= times.total_duration() {
        p.conditions.v1 + dir * p.j_max * (times.total_duration() - t).powi(2) / 2.
    } else {
        p.conditions.v1
    }
}

pub fn eval_acceleration(p: &SCurveParameters, t: f64) -> f64 {
    let times = &p.time_intervals;
    let dir = p.conditions.dir();
    if t < 0. {
        0.
    } else if t <= times.t_j1 {
        dir * p.j_max * t
    } else if t <= times.t_a - times.t_j1 {
        dir * p.a_lim_a
    } else if t <= times.t_a {
        dir * (-p.j_min) * (times.t_a - t)
    } else if t <= times.t_a + times.t_v {
        0.
    } else if t <= times.total_duration() - times.t_d + times.t_j2 {
        dir * (-p.j_max) * (t - times.total_duration() + times.t_d)
    } else if t <= times.total_duration() - times.t_j2 {
        dir * p.a_lim_d
    } else if t <= times.total_duration() {
        dir * (-p.j_max) * (times.total_duration() - t)
    } else {
        0.
    }
}

pub fn eval_jerk(p: &SCurveParameters, t: f64) -> f64 {
    let times = &p.time_intervals;
    let dir = p.conditions.dir();
    if t < times.t_j1 {
        dir * p.j_max
    } else if t <= times.t_a - times.t_j1 {
        0.
    } else if t <= times.t_a {
        dir * p.j_min
    } else if t <= times.t_a + times.t_v {
        0.
    } else if t <= times.total_duration() - times.t_d + times.t_j2 {
        dir * p.j_min
    } else if t <= times.total_duration() - times.t_j2 {
        0.
    } else {
        dir * p.j_max
    }
}

/// returns the S-Curve parameters and a function which maps time  [0,t] to Position, Velocity,
/// Acceleration or Jerk, depending on what you set as Derivative. Note that the acceleration
/// and velocity could be decreased if it is not possible to achieve them.
pub fn s_curve_generator(
    input_parameters: &SCurveInput,
    derivative: Derivative,
) -> (SCurveParameters, Box<dyn Fn(f64) -> f64>) {
    let times = input_parameters.calc_intervals();
    let params = SCurveParameters::new(&times, input_parameters);
    let params_clone = params.clone();

    match derivative {
        Derivative::Position => (
            params,
            Box::new(move |t: f64| eval_position(&params_clone, t)),
        ),
        Derivative::Velocity => (
            params,
            Box::new(move |t: f64| eval_velocity(&params_clone, t)),
        ),
        Derivative::Acceleration => (
            params,
            Box::new(move |t: f64| eval_acceleration(&params_clone, t)),
        ),
        Derivative::Jerk => (params, Box::new(move |t: f64| eval_jerk(&params_clone, t))),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SCurve {
    params: SCurveParameters,
    duration: f64,
    t_scale: f64,
    scale: f64,
}

impl SCurve {
    pub fn generate(
        q0: f64,
        q1: f64,
        v0: f64,
        v1: f64,
        max_vel: f64,
        max_accel: f64,
        max_jerk: f64,
    ) -> Self {
        SCurve::generate_scaled(q0, q1, v0, v1, max_vel, max_accel, max_jerk, 1.0, 1.0)
    }

    pub fn generate_scaled(
        q0: f64,
        q1: f64,
        v0: f64,
        v1: f64,
        max_vel: f64,
        max_accel: f64,
        max_jerk: f64,
        scale: f64,
        t_scale: f64,
    ) -> Self {
        let constraints = SCurveConstraints {
            max_jerk,
            max_acceleration: max_accel,
            max_velocity: max_vel,
        };
        let start_conditions = SCurveStartConditions { q0, q1, v0, v1 };
        let input_parameters = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input_parameters.calc_intervals();
        let duration = times.total_duration();
        SCurve {
            params: SCurveParameters::new(&times, &input_parameters),
            duration,
            scale,
            t_scale,
        }
    }

    /// Duration of the curve without time scaling
    pub fn duration_unscaled(&self) -> f64 {
        self.duration
    }

    /// Duration of the curve with time scaling
    pub fn duration(&self) -> f64 {
        self.duration / self.t_scale
    }

    /// Check if the curve is complete at a given time.
    pub fn complete_at(&self, t: f64) -> bool {
        t * self.t_scale >= self.duration
    }

    pub fn position_at(&self, t: f64) -> f64 {
        self.scale * eval_position(&self.params, t * self.t_scale)
    }

    pub fn velocity_at(&self, t: f64) -> f64 {
        self.scale * self.t_scale * eval_velocity(&self.params, t * self.t_scale)
    }

    pub fn acceleration_at(&self, t: f64) -> f64 {
        self.scale * self.t_scale * self.t_scale * eval_acceleration(&self.params, t * self.t_scale)
    }

    pub fn jerk_at(&self, t: f64) -> f64 {
        self.scale
            * self.t_scale
            * self.t_scale
            * self.t_scale
            * eval_jerk(&self.params, t * self.t_scale)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        s_curve_generator, Derivative, SCurve, SCurveConstraints, SCurveInput, SCurveParameters,
        SCurveStartConditions,
    };

    fn test_continuous(input: &SCurveInput, d: Derivative, constraining_param: f64) -> bool {
        let near_equal = |a: f64, b: f64, epsilon: f64| f64::abs(a - b) < epsilon;
        let (params, s_curve) = s_curve_generator(input, d);
        let mut p0 = s_curve(0.0);
        let datapoints = 1000;
        let margin = 100;
        let divisor = datapoints as f64;
        let e = constraining_param * 1.1 * params.time_intervals.total_duration() / divisor;
        for i in 0 - margin..(datapoints + 1) + margin {
            let p1 = s_curve(i as f64 * params.time_intervals.total_duration() / divisor);
            if near_equal(p0, p1, e) {
                p0 = p1;
            } else {
                println!("Out of limits, {p0}, {p1}, difference lim {e}");
                return false;
            }
        }
        return true;
    }

    fn test_continuous_pva(input: &SCurveInput) -> bool {
        if !test_continuous(input, Derivative::Position, input.constraints.max_velocity) {
            println!("Position not continuous");
            return false;
        }
        if !test_continuous(
            input,
            Derivative::Velocity,
            input.constraints.max_acceleration,
        ) {
            println!("Velocity not continuous");
            return false;
        }
        if !test_continuous(input, Derivative::Acceleration, input.constraints.max_jerk) {
            println!("Acceleration not continuous");
            return false;
        }
        return true;
    }

    #[test]
    fn timings_3_9() {
        let constraints = SCurveConstraints {
            max_jerk: 30.,
            max_acceleration: 10.0,
            max_velocity: 5.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let near_equal = |a: f64, b: f64, epsilon: f64| f64::abs(a - b) < epsilon;
        assert!(near_equal(times.t_a, 0.7333, 0.001));
        assert!(near_equal(times.t_v, 1.1433, 0.001));
        assert!(near_equal(times.t_d, 0.8333, 0.001));
        assert!(near_equal(times.t_j1, 0.333, 0.001));
        assert!(near_equal(times.t_j2, 0.333, 0.001));
    }

    #[test]
    fn timings_3_10() {
        let constraints = SCurveConstraints {
            max_jerk: 30.,
            max_acceleration: 10.0,
            max_velocity: 10.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let near_equal = |a: f64, b: f64, epsilon: f64| f64::abs(a - b) < epsilon;
        assert!(near_equal(times.t_a, 1.0747, 0.001));
        assert!(near_equal(times.t_v, 0., 0.001));
        assert!(near_equal(times.t_d, 1.1747, 0.001));
        assert!(near_equal(times.t_j1, 0.333, 0.001));
        assert!(near_equal(times.t_j2, 0.333, 0.001));
    }

    #[test]
    fn timings_3_11() {
        let constraints = SCurveConstraints {
            max_jerk: 30.,
            max_acceleration: 10.0,
            max_velocity: 10.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 7.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let near_equal = |a: f64, b: f64, epsilon: f64| f64::abs(a - b) < epsilon;
        assert!(near_equal(times.t_a, 0.4666, 0.001));
        assert!(near_equal(times.t_v, 0., 0.001));
        assert!(near_equal(times.t_d, 1.4718, 0.001));
        assert!(near_equal(times.t_j1, 0.2312, 0.001));
        assert!(near_equal(times.t_j2, 0.2321, 0.001));
    }

    #[test]
    fn timings_3_12() {
        let constraints = SCurveConstraints {
            max_jerk: 30.,
            max_acceleration: 10.0,
            max_velocity: 10.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 7.5,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let near_equal = |a: f64, b: f64, epsilon: f64| f64::abs(a - b) < epsilon;
        assert!(near_equal(times.t_a, 0., 0.001));
        assert!(near_equal(times.t_v, 0., 0.001));
        assert!(near_equal(times.t_d, 2.6667, 0.001));
        assert!(near_equal(times.t_j1, 0., 0.001));
        assert!(near_equal(times.t_j2, 0.0973, 0.001));
    }

    #[test]
    fn simple_curve() {
        let constraints = SCurveConstraints {
            max_jerk: 30.,
            max_acceleration: 10.0,
            max_velocity: 5.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let s_curve_tmp = s_curve_generator(&input, Derivative::Position);
        let s_curve = s_curve_tmp.1;
        let params = s_curve_tmp.0;
        for i in 0..101 {
            println!(
                "{}",
                s_curve(i as f64 * params.time_intervals.total_duration() / 100.)
            );
        }
    }

    #[test]
    fn simple_curve_reverse() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 0.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();

        assert_eq!(times.total_duration(), 5.5);
    }

    #[test]
    fn simple_curve_reverse_no_phase_3() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 5.,
            q1: 0.,
            v0: 0.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();

        assert_eq!(times.total_duration(), 3.8984532382775527);
    }

    #[test]
    fn test_is_a_min_not_reached() {
        let constraints = SCurveConstraints {
            max_jerk: 0.03,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 0.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn test_is_max_acceleration_not_reached() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 1.,
            q1: 0.,
            v0: 0.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn vlim_move_pos_v0_right_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let params = SCurveParameters::new(&times, &input);

        assert_eq!(params.v_lim, 3.0);
    }

    #[test]
    fn vlim_move_pos_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: -1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let params = SCurveParameters::new(&times, &input);

        assert_eq!(params.v_lim, 3.0);
    }

    #[test]
    fn vlim_move_neg_v0_right_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: -1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let params = SCurveParameters::new(&times, &input);

        assert_eq!(params.v_lim, 3.0);
    }

    #[test]
    fn vlim_move_neg_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();
        let params = SCurveParameters::new(&times, &input);

        assert_eq!(params.v_lim, 3.0);
    }

    #[test]
    fn t_v_pos_move_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: -1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();

        assert_eq!(times.t_v, 1.3611111111111114);
    }

    #[test]
    fn t_v_neg_move_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();

        assert_eq!(times.t_v, 1.3611111111111114);
    }

    #[test]
    fn pos_move_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: -1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();

        assert_eq!(times.total_duration(), 6.194444444444445);
    }

    #[test]
    fn neg_move_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let times = input.calc_intervals();

        assert_eq!(times.total_duration(), 6.194444444444445);
    }

    #[test]
    fn continuous_simple() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 0.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_nonzero_v1() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 0.,
            v1: 1.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_neg_nonzero_v1() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 0.,
            v1: -1.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_neg_nonzero_v1_wrongdirection() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 0.,
            v1: 1.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_nonzero_v1_wrongdirection() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 0.,
            v1: -1.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_nonzero_v0() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_nonzero_v0_wrongdir() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_negative() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 0.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_negative_nonzero_v0() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: -1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn continuous_negative_nonzero_v0_wrong_direction() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 10.,
            q1: 0.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(test_continuous_pva(&input));
    }

    #[test]
    fn calc_intervals_should_get_same_result_for_opposite_directions() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let constraints_2 = constraints.clone();
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: -1.,
            v1: 2.,
        };
        let start_conditions_2 = SCurveStartConditions {
            q0: 0.,
            q1: -10.,
            v0: 1.,
            v1: -2.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let input_2 = SCurveInput {
            constraints: constraints_2,
            start_conditions: start_conditions_2,
        };
        let times = input.calc_intervals();
        let times_2 = input_2.calc_intervals();

        assert!(times.t_j1 == times_2.t_j1);
        assert!(times.t_a == times_2.t_a);
        assert!(times.t_v == times_2.t_v);
        assert!(times.t_j2 == times_2.t_j2);
        assert!(times.t_d == times_2.t_d);
    }

    #[test]
    fn calc_intervals_should_get_same_result_for_opposite_directions_vmax_not_reached() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let constraints_2 = constraints.clone();
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 5.,
            v0: -1.,
            v1: 2.,
        };
        let start_conditions_2 = SCurveStartConditions {
            q0: 0.,
            q1: -5.,
            v0: 1.,
            v1: -2.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };
        let input_2 = SCurveInput {
            constraints: constraints_2,
            start_conditions: start_conditions_2,
        };
        let times = input.calc_intervals();
        let times_2 = input_2.calc_intervals();

        assert!(times.t_j1 == times_2.t_j1);
        assert!(times.t_a == times_2.t_a);
        assert!(times.t_v == times_2.t_v);
        assert!(times.t_j2 == times_2.t_j2);
        assert!(times.t_d == times_2.t_d);
    }

    #[test]
    fn is_trajectory_feasible_basic() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 10.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(input.is_trajectory_feasible());
    }

    #[test]
    fn is_trajectory_feasible_v_max_not_reached() {
        let constraints = SCurveConstraints {
            max_jerk: 3.,
            max_acceleration: 2.0,
            max_velocity: 3.,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.,
            q1: 4.,
            v0: 1.,
            v1: 0.,
        };
        let input = SCurveInput {
            constraints,
            start_conditions,
        };

        assert!(input.is_trajectory_feasible());
    }

    #[test]
    fn time_scaling_doubles_duration() {
        let constraints = SCurveConstraints {
            max_jerk: 3.0,
            max_acceleration: 2.0,
            max_velocity: 3.0,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.0,
            q1: 10.0,
            v0: 0.0,
            v1: 0.0,
        };

        // Create unscaled curve
        let s_curve = SCurve::generate(
            start_conditions.q0,
            start_conditions.q1,
            start_conditions.v0,
            start_conditions.v1,
            constraints.max_velocity,
            constraints.max_acceleration,
            constraints.max_jerk,
        );
        let original_duration = s_curve.duration();
        let final_pos = s_curve.position_at(original_duration);

        // Create curve with doubled duration (t_scale = 0.5)
        let scaled_curve = SCurve::generate_scaled(
            start_conditions.q0,
            start_conditions.q1,
            start_conditions.v0,
            start_conditions.v1,
            constraints.max_velocity,
            constraints.max_acceleration,
            constraints.max_jerk,
            1.0, // position scale
            0.5, // time scale
        );

        assert!((scaled_curve.duration() - 2.0 * original_duration).abs() < 1e-10);
        assert!((scaled_curve.position_at(2.0 * original_duration) - final_pos).abs() < 1e-10);
    }

    #[test]
    fn time_scaling_scales_velocity() {
        let constraints = SCurveConstraints {
            max_jerk: 3.0,
            max_acceleration: 2.0,
            max_velocity: 3.0,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.0,
            q1: 10.0,
            v0: 0.0,
            v1: 0.0,
        };

        // Create unscaled curve
        let s_curve = SCurve::generate(
            start_conditions.q0,
            start_conditions.q1,
            start_conditions.v0,
            start_conditions.v1,
            constraints.max_velocity,
            constraints.max_acceleration,
            constraints.max_jerk,
        );

        // Create curve with halved velocities (t_scale = 0.5)
        let scaled_curve = SCurve::generate_scaled(
            start_conditions.q0,
            start_conditions.q1,
            start_conditions.v0,
            start_conditions.v1,
            constraints.max_velocity,
            constraints.max_acceleration,
            constraints.max_jerk,
            1.0, // position scale
            0.5, // time scale
        );

        // Check velocity at several points
        let test_times = vec![0.5, 1.0, 1.5, 2.0];
        for t in test_times {
            let unscaled_vel = s_curve.velocity_at(t);
            let scaled_vel = scaled_curve.velocity_at(2.0 * t);
            assert!((scaled_vel - 0.5 * unscaled_vel).abs() < 1e-10);
        }
    }

    #[test]
    fn time_scaling_scales_acceleration() {
        let constraints = SCurveConstraints {
            max_jerk: 3.0,
            max_acceleration: 2.0,
            max_velocity: 3.0,
        };
        let start_conditions = SCurveStartConditions {
            q0: 0.0,
            q1: 10.0,
            v0: 0.0,
            v1: 0.0,
        };

        // Create unscaled curve
        let s_curve = SCurve::generate(
            start_conditions.q0,
            start_conditions.q1,
            start_conditions.v0,
            start_conditions.v1,
            constraints.max_velocity,
            constraints.max_acceleration,
            constraints.max_jerk,
        );

        // Create curve with quartered accelerations (t_scale = 0.5)
        let scaled_curve = SCurve::generate_scaled(
            start_conditions.q0,
            start_conditions.q1,
            start_conditions.v0,
            start_conditions.v1,
            constraints.max_velocity,
            constraints.max_acceleration,
            constraints.max_jerk,
            1.0, // position scale
            0.5, // time scale
        );

        // Check acceleration at several points
        let test_times = vec![0.5, 1.0, 1.5, 2.0];
        for t in test_times {
            let unscaled_acc = s_curve.acceleration_at(t);
            let scaled_acc = scaled_curve.acceleration_at(2.0 * t);
            assert!((scaled_acc - 0.25 * unscaled_acc).abs() < 1e-10);
        }
    }
}
